use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "brain", about = "AI agent engine CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
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

#[derive(Subcommand)]
pub enum SessionsAction {
    /// List past sessions
    List,
    /// Resume an existing session by ID
    Resume {
        /// Session ULID
        id: String,
    },
}

#[derive(Subcommand)]
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
