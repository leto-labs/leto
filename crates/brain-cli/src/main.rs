mod commands;
mod provider;

use std::sync::Arc;

use anyhow::{Context, Result};
use clap::Parser;
use futures::StreamExt;
use ulid::Ulid;

use brain_core::*;
use brain_server::{BrainApi, BrainServer};
use brain_stores::brain_home;

use commands::{Cli, Commands, CredentialsAction, SessionsAction};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();
    let command = cli.command;

    if matches!(command, Some(Commands::Acp)) {
        brain_acp::run_stdio()
            .await
            .context("failed to run mock ACP stdio server")?;
        return Ok(());
    }

    let cwd = std::env::current_dir().context("failed to get current directory")?;

    let config = resolve_config(&cwd);

    let store: Arc<dyn Store> = Arc::new(
        FileStore::new(brain_home())
            .await
            .context("failed to initialize FileStore")?,
    );

    let pool = Arc::new(CredentialPool::new(
        store.clone() as Arc<dyn CredentialStore>,
        Arc::new(Fallback::new()),
    ));

    let provider = provider::build_provider(&config, pool).await;
    let tools = native_tools();
    let brain = Brain::new(provider, store, Arc::new(SimpleLoop), tools);
    let server = BrainServer::new(brain);
    let client = server.client();

    let project_name = cwd
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "brain".to_owned());

    let project = find_or_create_project(&client, &project_name, &cwd, config).await?;

    match command {
        Some(Commands::Acp) => unreachable!("ACP command is handled before runtime initialization"),
        Some(Commands::Credentials { action }) => match action {
            CredentialsAction::Add {
                provider,
                api_key,
                id,
            } => cmd_credentials_add(&client, &provider, &api_key, &id).await,
            CredentialsAction::Login { provider, device } => {
                cmd_credentials_login(&client, &provider, device).await
            }
            CredentialsAction::List => cmd_credentials_list(&client).await,
            CredentialsAction::Remove { provider, id } => {
                cmd_credentials_remove(&client, &provider, &id).await
            }
        },
        Some(Commands::Sessions { action }) => match action {
            SessionsAction::List => cmd_sessions_list(&client, project.id).await,
            SessionsAction::Resume { id } => cmd_sessions_resume(&client, &project, &id).await,
        },
        None => cmd_chat(&client, project.id, None).await,
    }
}

fn resolve_config(cwd: &std::path::Path) -> ProjectConfig {
    let mut config = brain_config::resolve_fs_config(cwd).unwrap_or_else(|e| {
        tracing::warn!("config resolution failed, using defaults: {e}");
        ProjectConfig::default()
    });

    if config.agent.system_prompt.is_none()
        && let Ok(Some(agents_md)) = brain_config::load_root_agents_md(cwd)
    {
        config.agent.system_prompt = Some(agents_md);
    }

    config
}

async fn find_or_create_project(
    client: &Arc<dyn BrainApi>,
    name: &str,
    root: &std::path::Path,
    config: ProjectConfig,
) -> Result<Project> {
    let projects = client.list_projects().await?;
    if let Some(existing) = projects.iter().find(|p| p.name.as_deref() == Some(name)) {
        return Ok(existing.clone());
    }

    let project = Project::new(Some(name.to_owned()), Some(root.to_owned()), config);
    let project = client.create_project(project).await?;
    Ok(project)
}

async fn cmd_chat(
    client: &Arc<dyn BrainApi>,
    project_id: ProjectId,
    resume_session: Option<Ulid>,
) -> Result<()> {
    let session = if let Some(id) = resume_session {
        client.get_session(id).await?
    } else {
        client.create_session(project_id).await?
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

        let mut stream = client.send_message_stream(session.id, input).await?;

        while let Some(event) = stream.next().await {
            match &event {
                Event::Token { delta } => {
                    print!("{delta}");
                }
                Event::ToolCallStart { name, .. } => {
                    eprintln!("\n[tool: {name}]");
                }
                Event::ToolCallDone { result, .. } => {
                    let preview: String = result.chars().take(200).collect();
                    eprintln!("[result: {preview}...]");
                }
                Event::Error { message, .. } => {
                    eprintln!("\nerror: {message}");
                }
                Event::TurnDone { .. } => {
                    println!();
                    break;
                }
                _ => {}
            }
        }
    }

    Ok(())
}

async fn cmd_sessions_list(client: &Arc<dyn BrainApi>, project_id: ProjectId) -> Result<()> {
    let sessions = client.list_sessions(project_id).await?;

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
    client: &Arc<dyn BrainApi>,
    project: &Project,
    id_str: &str,
) -> Result<()> {
    let session_id: Ulid = id_str
        .parse()
        .context("invalid session ID (expected a ULID)")?;

    let session = client
        .get_session(session_id)
        .await
        .map_err(|e| anyhow::anyhow!("session not found: {e}"))?;

    if session.project_id != project.id {
        anyhow::bail!(
            "session {} does not belong to project {:?}",
            session_id,
            project.name
        );
    }

    let messages = client.list_messages(session_id).await?;
    if !messages.is_empty() {
        println!(
            "--- Resuming session {} ({} messages) ---",
            session_id,
            messages.len()
        );
    }

    cmd_chat(client, project.id, Some(session_id)).await
}

async fn cmd_credentials_add(
    client: &Arc<dyn BrainApi>,
    provider: &str,
    api_key: &str,
    id: &str,
) -> Result<()> {
    let entry = CredentialEntry::api_key(id, api_key);
    client.save_credential(provider, entry).await?;
    println!("Saved credential '{id}' for provider '{provider}'.");
    Ok(())
}

async fn cmd_credentials_list(client: &Arc<dyn BrainApi>) -> Result<()> {
    let credentials = client.list_credentials().await?;

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

async fn cmd_credentials_login(
    client: &Arc<dyn BrainApi>,
    provider: &str,
    device: bool,
) -> Result<()> {
    #[cfg(feature = "openai-oauth")]
    {
        use brain_core::OpenAiOAuthPreset;

        let preset = match provider {
            "openai-oauth" | "openai_oauth" => &OpenAiOAuthPreset::OPENAI,
            other => anyhow::bail!("unknown OAuth provider '{other}'. Available: openai-oauth"),
        };

        let entry = if device {
            println!("Starting device code flow...\n");
            oauth_device_flow(client, preset).await?
        } else {
            println!("Starting browser login flow...\n");
            oauth_browser_flow(client, preset).await?
        };

        println!(
            "\nLogin successful! Credential '{}' saved for '{}'.",
            entry.id, provider
        );
        Ok(())
    }

    #[cfg(not(feature = "openai-oauth"))]
    {
        let _ = (client, provider, device);
        anyhow::bail!("OAuth support not compiled in. Rebuild with --features openai-oauth");
    }
}

#[cfg(feature = "openai-oauth")]
async fn oauth_device_flow(
    client: &Arc<dyn BrainApi>,
    preset: &brain_core::OpenAiOAuthPreset,
) -> Result<CredentialEntry> {
    use brain_core::device_flow::{self, DeviceFlowConfig};

    let http = reqwest::Client::new();
    let flow_config = DeviceFlowConfig {
        device_code_url: preset.device_code_url.to_owned(),
        device_token_url: preset.device_token_url.to_owned(),
        token_url: preset.token_url.to_owned(),
        client_id: preset.client_id.to_owned(),
        redirect_uri: preset.device_redirect_uri.to_owned(),
    };

    let creds = device_flow::run(
        &http,
        &flow_config,
        preset.device_verification_url,
        |prompt| {
            println!("Go to:      {}", prompt.verification_url);
            println!("Enter code: {}", prompt.user_code);
            println!("\nWaiting for authorization...");
        },
    )
    .await?;

    let entry = CredentialEntry::oauth(creds);
    client.save_credential(preset.name, entry.clone()).await?;
    Ok(entry)
}

#[cfg(feature = "openai-oauth")]
async fn oauth_browser_flow(
    client: &Arc<dyn BrainApi>,
    preset: &brain_core::OpenAiOAuthPreset,
) -> Result<CredentialEntry> {
    use brain_core::browser_flow::{self, BrowserFlowConfig};

    let http = reqwest::Client::new();
    let flow_config = BrowserFlowConfig {
        authorize_url: preset.authorize_url.to_owned(),
        token_url: preset.token_url.to_owned(),
        client_id: preset.client_id.to_owned(),
        scopes: preset.scopes.to_owned(),
        callback_port: preset.callback_port,
        timeout: std::time::Duration::from_secs(300),
    };

    let creds = browser_flow::run(&http, &flow_config, |prompt| {
        println!("Open: {}\n", prompt.url);
        println!("Complete authorization in your browser.");
        println!("Waiting for callback on port {}...", preset.callback_port);
    })
    .await?;

    let entry = CredentialEntry::oauth(creds);
    client.save_credential(preset.name, entry.clone()).await?;
    Ok(entry)
}

async fn cmd_credentials_remove(
    client: &Arc<dyn BrainApi>,
    provider: &str,
    id: &str,
) -> Result<()> {
    client.delete_credential(provider, id).await?;
    println!("Removed credential '{id}' from provider '{provider}'.");
    Ok(())
}
