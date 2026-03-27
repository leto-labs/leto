//! Shared credential and pooling primitives for standalone provider crates.

mod context;
mod health;
mod pool;
mod strategy;
mod types;

pub use context::SelectionContext;
pub use health::{CredentialFailure, CredentialFailureRecord, CredentialHealth};
pub use pool::{CredentialPool, CredentialPoolError};
pub use strategy::{Fallback, SelectionStrategy, StickyRoundRobin};
pub use types::{CredentialEntry, CredentialMaterial, ResolvedCredential};
