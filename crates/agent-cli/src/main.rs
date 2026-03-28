mod commands;
mod exec_mode;

use std::sync::Arc;

use agent_core::{AgentCore, AgentCoreNative, CoreError, CoreEvent};
use agent_runtime::{BlockDelta, Message, RuntimeEvent};
use agent_store::{
    CredentialEntry, FileStore, OAuthCredentials, Project, ProjectId, ProviderCredential, Store,
    agent_home,
};
use anyhow::{Context, Result};
use clap::Parser;
use futures::StreamExt;
use provider::MockProvider;
use provider_openai::{OAuthFlow, OpenAiOAuthPreset};
use ulid::Ulid;

use crate::commands::{Cli, Commands, CredentialsAction, SessionsAction};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();
    let command = cli.command;

    if let Some(Commands::Exec(exec)) = command.clone() {
        exec_mode::run_exec(exec)
            .await
            .context("failed to run exec command")?;
        return Ok(());
    }

    let cwd = std::env::current_dir().context("failed to get current directory")?;
    let store: Arc<dyn Store> = Arc::new(
        FileStore::new(agent_home())
            .await
            .context("failed to initialize file store")?,
    );
    let core = build_core(store)
        .await
        .context("failed to initialize embedded AgentCore")?;

    match command {
        Some(Commands::Acp) => agent_acp::run_stdio(core)
            .await
            .context("failed to run ACP stdio compatibility alias"),
        Some(Commands::Credentials { action }) => match action {
            CredentialsAction::Add {
                provider,
                api_key,
                id,
            } => cmd_credentials_add(&core, &provider, &api_key, &id).await,
            CredentialsAction::Login { provider, device } => {
                cmd_credentials_login(&core, &provider, device).await
            }
            CredentialsAction::List => cmd_credentials_list(&core).await,
            CredentialsAction::Remove { provider, id } => {
                cmd_credentials_remove(&core, &provider, &id).await
            }
        },
        Some(Commands::Sessions { action }) => {
            let project = core
                .resolve_or_create_project(cwd.clone())
                .await
                .context("failed to resolve project")?;
            match action {
                SessionsAction::List => cmd_sessions_list(&core, project.id).await,
                SessionsAction::Resume { id } => cmd_sessions_resume(&core, &project, &id).await,
            }
        }
        None => {
            let project = core
                .resolve_or_create_project(cwd)
                .await
                .context("failed to resolve project")?;
            cmd_chat(&core, project.id, None).await
        }
        Some(Commands::Exec(_)) => {
            unreachable!("exec command is handled before core initialization")
        }
    }
}

async fn build_core(store: Arc<dyn Store>) -> Result<AgentCoreNative, CoreError> {
    match AgentCoreNative::build_default_local(store.clone()).await {
        Ok(core) => Ok(core),
        Err(CoreError::NoProvidersRegistered) => Ok(AgentCoreNative::builder(store)
            .with_provider("mock", Arc::new(MockProvider::new()))
            .default_provider("mock")
            .build()
            .await?),
        Err(error) => return Err(error),
    }
}

async fn cmd_chat(
    core: &dyn AgentCore,
    project_id: ProjectId,
    resume_session: Option<Ulid>,
) -> Result<()> {
    let session = if let Some(id) = resume_session {
        core.session(id).await?
    } else {
        core.create_session(project_id).await?
    };

    loop {
        eprint!("> ");
        let input = tokio::task::spawn_blocking(|| {
            use std::io::Write;
            std::io::stderr().flush().ok();
            let mut buf = String::new();
            match std::io::stdin().read_line(&mut buf) {
                Ok(0) | Err(_) => None,
                Ok(_) => Some(buf),
            }
        })
        .await?;

        let Some(input) = input else { break };
        let input = input.trim();
        if input.is_empty() {
            continue;
        }
        if input == "/quit" || input == "/exit" {
            break;
        }

        let mut stream = core
            .turn(session.id, vec![Message::user_text(input)])
            .await
            .context("failed to start turn")?;

        while let Some(event) = stream.next().await {
            match event {
                CoreEvent::Turn {
                    event: RuntimeEvent::OutputBlockDelta { delta, .. },
                    ..
                } => {
                    if let BlockDelta::Text { text } = delta {
                        print!("{text}");
                    }
                }
                CoreEvent::Turn {
                    event: RuntimeEvent::ToolCallStarted { call },
                    ..
                } => {
                    eprintln!("\n[tool: {}]", call.name);
                }
                CoreEvent::Turn {
                    event: RuntimeEvent::ToolCallFinished { result, .. },
                    ..
                } => {
                    let preview = result.output.to_string();
                    let preview: String = preview.chars().take(200).collect();
                    eprintln!("[result: {preview}]");
                }
                CoreEvent::Turn {
                    event: RuntimeEvent::Error { message, .. },
                    ..
                } => {
                    eprintln!("\nerror: {message}");
                }
                CoreEvent::Turn {
                    event: RuntimeEvent::TurnFinished { .. },
                    ..
                } => {
                    println!();
                    break;
                }
                CoreEvent::TurnCancelled { .. } => {
                    eprintln!("\nturn cancelled");
                    break;
                }
                _ => {}
            }
        }
    }

    Ok(())
}

async fn cmd_sessions_list(core: &dyn AgentCore, project_id: ProjectId) -> Result<()> {
    let sessions = core.sessions_for_project(project_id).await?;
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

async fn cmd_sessions_resume(core: &dyn AgentCore, project: &Project, id_str: &str) -> Result<()> {
    let session_id: Ulid = id_str
        .parse()
        .context("invalid session ID (expected a ULID)")?;
    let session = core
        .session(session_id)
        .await
        .map_err(|error| anyhow::anyhow!("session not found: {error}"))?;

    if session.project_id != project.id {
        anyhow::bail!(
            "session {} does not belong to project {:?}",
            session_id,
            project.name
        );
    }

    cmd_chat(core, project.id, Some(session.id)).await
}

async fn cmd_credentials_add(
    core: &dyn AgentCore,
    provider: &str,
    api_key: &str,
    id: &str,
) -> Result<()> {
    let entry = CredentialEntry::api_key(id, api_key);
    save_credential(core, provider, entry).await?;
    println!("Saved credential '{id}' for provider '{provider}'.");
    Ok(())
}

async fn cmd_credentials_list(core: &dyn AgentCore) -> Result<()> {
    let credentials = core.store().credentials().list().await?;
    if credentials.is_empty() {
        println!("No credentials stored.");
        println!("Add one with: agent credentials add <provider> <api-key>");
        return Ok(());
    }

    for ((provider_name, _), entry) in &credentials {
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

async fn cmd_credentials_login(core: &dyn AgentCore, provider: &str, device: bool) -> Result<()> {
    let preset = match provider {
        "openai-oauth" | "openai_oauth" => OpenAiOAuthPreset::OPENAI,
        other => anyhow::bail!("unknown OAuth provider '{other}'. Available: openai-oauth"),
    };

    let entry = if device {
        println!("Starting device code flow...\n");
        let creds = OAuthFlow::login_device(&preset, |prompt| {
            println!("Go to:      {}", prompt.verification_url);
            println!("Enter code: {}", prompt.user_code);
            println!("\nWaiting for authorization...");
        })
        .await?;
        CredentialEntry::oauth("oauth", oauth_credentials_from_provider(creds))
    } else {
        println!("Starting browser login flow...\n");
        let callback_port = preset.callback_port;
        let creds = OAuthFlow::login_browser(&preset, |prompt| {
            println!("Open: {}\n", prompt.url);
            println!("Complete authorization in your browser.");
            println!("Waiting for callback on port {}...", callback_port);
        })
        .await?;
        CredentialEntry::oauth("oauth", oauth_credentials_from_provider(creds))
    };

    save_credential(core, preset.name, entry.clone()).await?;
    println!(
        "\nLogin successful! Credential '{}' saved for '{}'.",
        entry.id, preset.name
    );
    Ok(())
}

fn oauth_credentials_from_provider(
    creds: provider_openai::OpenAiOAuthCredentials,
) -> OAuthCredentials {
    OAuthCredentials {
        access_token: creds.access_token,
        refresh_token: creds.refresh_token,
        client_id: creds.client_id,
        token_endpoint: creds.token_endpoint,
        account_id: creds.account_id,
        token_type: creds.token_type,
        expires_at: creds.expires_at,
        scopes: creds.scopes,
    }
}

async fn cmd_credentials_remove(core: &dyn AgentCore, provider: &str, id: &str) -> Result<()> {
    core.store()
        .credentials()
        .delete((provider.to_owned(), id.to_owned()))
        .await?;
    println!("Removed credential '{id}' from provider '{provider}'.");
    Ok(())
}

async fn save_credential(
    core: &dyn AgentCore,
    provider: &str,
    entry: CredentialEntry,
) -> Result<()> {
    let key = (provider.to_owned(), entry.id.clone());
    match core.store().credentials().get(key.clone()).await {
        Ok(_) => {
            core.store().credentials().update(key, entry).await?;
        }
        Err(_) => {
            core.store().credentials().create(key, entry).await?;
        }
    }
    Ok(())
}
