//! Standalone llama.cpp-backed local provider implementation.
//!
//! The crate exposes local model configuration/preset types plus an optional
//! shared [`provider::Provider`] adapter behind the `llamacpp` feature.

pub mod config;
pub mod presets;
#[cfg(feature = "llamacpp")]
pub mod provider_impl;

pub use config::LlamaCppConfig;
pub use presets::{LlamaCppArtifacts, LlamaCppModelPreset};
#[cfg(feature = "llamacpp")]
pub use provider_impl::LlamaCppProvider;
