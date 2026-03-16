mod commands;
mod provider;

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use clap::Parser;
use ulid::Ulid;

use brain_core::*;

use commands::{Cli, Commands, CredentialsAction, SessionsAction};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();
    let cwd = std::env::current_dir().context("failed to get current directory")?;

    let config = resolve_config(&cwd);

    let store_root = discover_store_root(&cwd);
    let store: Arc<dyn Store> = Arc::new(
        FileStore::new(&store_root)
            .await
            .context("failed to initialize FileStore")?,
    );

    let pool = Arc::new(CredentialPool::new(
        store.clone() as Arc<dyn CredentialStore>,
        Arc::new(Fallback::new()),
    ));

    match cli.command {
        Some(Commands::Credentials { action }) => match action {
            CredentialsAction::Add {
                provider,
                api_key,
                id,
            } => cmd_credentials_add(store, &provider, &api_key, &id).await,
            CredentialsAction::List => cmd_credentials_list(store).await,
            CredentialsAction::Remove { provider, id } => {
                cmd_credentials_remove(store, &provider, &id).await
            }
        },
        other => {
            let project_name = cwd
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "brain".to_owned());

            let project =
                find_or_create_project(store.clone(), &project_name, &cwd, config.clone()).await?;

            match other {
                Some(Commands::Sessions { action }) => match action {
                    SessionsAction::List => cmd_sessions_list(store, project.id).await,
                    SessionsAction::Resume { id } => {
                        cmd_sessions_resume(store, pool, &config, &project, &id).await
                    }
                },
                None => cmd_chat(&project, store, pool, &config, None).await,
                Some(Commands::Credentials { .. }) => unreachable!(),
            }
        }
    }
}

fn resolve_config(cwd: &std::path::Path) -> ProjectConfig {
    let mut config = brain_config::resolve_fs_config(cwd).unwrap_or_else(|e| {
        tracing::warn!("config resolution failed, using defaults: {e}");
        ProjectConfig::default()
    });

    if config.agent.system_prompt.is_none() {
        if let Ok(Some(agents_md)) = brain_config::load_root_agents_md(cwd) {
            config.agent.system_prompt = Some(agents_md);
        }
    }

    config
}

async fn find_or_create_project(
    store: Arc<dyn Store>,
    name: &str,
    root: &std::path::Path,
    config: ProjectConfig,
) -> Result<Project> {
    let projects = store.project_list().await?;
    if let Some(existing) = projects.iter().find(|p| p.name.as_deref() == Some(name)) {
        return Ok(existing.clone());
    }

    let project = Project::new(Some(name.to_owned()), Some(root.to_owned()), config);
    let project = store.project_create(project).await?;
    Ok(project)
}

async fn cmd_chat(
    project: &Project,
    store: Arc<dyn Store>,
    pool: Arc<CredentialPool>,
    config: &ProjectConfig,
    resume_session: Option<Ulid>,
) -> Result<()> {
    let provider = provider::build_provider(config, pool).await;
    let tools = native_tools();
    let brain = Brain::new(provider, store, Arc::new(SimpleLoop), tools);
    let transport = CliTransport::new();

    brain
        .run(project, &transport, resume_session)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))
}

async fn cmd_sessions_list(store: Arc<dyn Store>, project_id: ProjectId) -> Result<()> {
    let sessions = store.session_list(project_id).await?;

    if sessions.is_empty() {
        println!("No sessions found.");
        return Ok(());
    }

    for session in &sessions {
        let title = session.title.as_deref().unwrap_or("(untitled)");
        let date = session.updated_at.format("%Y-%m-%d %H:%M");
        println!("{id}  {title}  {date}", id = session.id);
    }

    Ok(())
}

async fn cmd_sessions_resume(
    store: Arc<dyn Store>,
    pool: Arc<CredentialPool>,
    config: &ProjectConfig,
    project: &Project,
    id_str: &str,
) -> Result<()> {
    let session_id: Ulid = id_str
        .parse()
        .context("invalid session ID (expected a ULID)")?;

    let session = store
        .session_get(session_id)
        .await
        .map_err(|e| anyhow::anyhow!("session not found: {e}"))?;

    if session.project_id != project.id {
        anyhow::bail!(
            "session {} does not belong to project {:?}",
            session_id,
            project.name
        );
    }

    let messages = store.message_list(session_id).await?;
    if !messages.is_empty() {
        println!(
            "--- Resuming session {} ({} messages) ---",
            session_id,
            messages.len()
        );
    }

    cmd_chat(project, store, pool, config, Some(session_id)).await
}

async fn cmd_credentials_add(
    store: Arc<dyn Store>,
    provider: &str,
    api_key: &str,
    id: &str,
) -> Result<()> {
    let entry = CredentialEntry::api_key(id, api_key);
    store.credential_save(provider, &entry).await?;
    println!("Saved credential '{id}' for provider '{provider}'.");
    Ok(())
}

async fn cmd_credentials_list(store: Arc<dyn Store>) -> Result<()> {
    let credentials = store.credential_list().await?;

    if credentials.is_empty() {
        println!("No credentials stored.");
        println!("Add one with: brain credentials add <provider> <api-key>");
        return Ok(());
    }

    for (provider_name, entry) in &credentials {
        let kind = match &entry.credential {
            ProviderCredential::ApiKey { api_key } => {
                let masked = if api_key.len() > 8 {
                    format!("{}...{}", &api_key[..4], &api_key[api_key.len() - 4..])
                } else {
                    "****".to_owned()
                };
                format!("api_key ({masked})")
            }
            ProviderCredential::OAuth(_) => "oauth".to_owned(),
        };
        let status = if entry.enabled { "enabled" } else { "disabled" };
        println!("{provider_name}/{id}  {kind}  [{status}]", id = entry.id);
    }

    Ok(())
}

async fn cmd_credentials_remove(
    store: Arc<dyn Store>,
    provider: &str,
    id: &str,
) -> Result<()> {
    store.credential_delete(provider, id).await?;
    println!("Removed credential '{id}' from provider '{provider}'.");
    Ok(())
}

fn discover_store_root(cwd: &std::path::Path) -> PathBuf {
    let mut cursor = cwd.to_owned();
    loop {
        let agents_dir = cursor.join(".agents");
        if agents_dir.is_dir() {
            return agents_dir.join("store");
        }
        if cursor.join(".git").is_dir() {
            break;
        }
        match cursor.parent() {
            Some(parent) if parent != cursor => {
                cursor = parent.to_owned();
            }
            _ => break,
        }
    }
    cwd.join(".agents").join("store")
}
