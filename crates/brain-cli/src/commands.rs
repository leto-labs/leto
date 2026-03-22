use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(name = "brain", about = "AI agent engine CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Clone)]
pub enum Commands {
    /// Run the ACP compatibility alias (canonical binaries live in brain-acp)
    Acp,
    /// Execute one non-interactive agent run for benchmark harnesses
    Exec(ExecCommand),
    /// Manage sessions
    Sessions {
        #[command(subcommand)]
        action: SessionsAction,
    },
    /// Manage provider credentials
    Credentials {
        #[command(subcommand)]
        action: CredentialsAction,
    },
}

#[derive(Args, Clone)]
pub struct ExecCommand {
    /// Workspace root / current working directory for the run
    #[arg(long)]
    pub cwd: PathBuf,
    /// Model ID to use for the run (provider-qualified names are accepted)
    #[arg(long)]
    pub model: String,
    /// Registered loop name to use for the run
    #[arg(long = "loop")]
    pub loop_name: String,
    /// Provider name to use for the run
    #[arg(long)]
    pub provider: String,
    /// Directory where run artifacts will be written
    #[arg(long)]
    pub output_dir: PathBuf,
    /// API key to inject for the selected provider
    #[arg(long)]
    pub api_key: Option<String>,
    /// Optional base URL override for OpenAI-compatible providers
    #[arg(long)]
    pub base_url: Option<String>,
    /// Optional API surface override: auto, responses, or chat-completions
    #[arg(long)]
    pub api_surface: Option<String>,
    /// Instruction to execute
    #[arg(required = true)]
    pub instruction: String,
}

#[derive(Subcommand, Clone)]
pub enum SessionsAction {
    /// List past sessions
    List,
    /// Resume an existing session by ID
    Resume {
        /// Session ULID
        id: String,
    },
}

#[derive(Subcommand, Clone)]
pub enum CredentialsAction {
    /// Add an API key for a provider
    Add {
        /// Provider name (e.g. openai, groq, deepseek)
        provider: String,
        /// API key value
        api_key: String,
        /// Optional credential ID (defaults to "default")
        #[arg(long, default_value = "default")]
        id: String,
    },
    /// Log in to a provider via OAuth (browser or device code flow)
    Login {
        /// Provider name (currently: openai-oauth)
        provider: String,
        /// Use device code flow instead of browser
        #[arg(long)]
        device: bool,
    },
    /// List all stored credentials
    List,
    /// Remove a credential
    Remove {
        /// Provider name
        provider: String,
        /// Credential ID to remove
        id: String,
    },
}
