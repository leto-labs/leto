//! Standalone mistral.rs-backed local provider implementation.
//!
//! The crate exposes local model configuration/preset types plus an optional
//! shared [`provider::Provider`] adapter behind the `mistralrs` feature.

pub mod config;
pub mod presets;
#[cfg(feature = "mistralrs")]
pub mod provider_impl;

pub use config::{DevicePreference, MistralRsConfig};
pub use presets::{MistralRsArtifacts, MistralRsModelPreset};
#[cfg(feature = "mistralrs")]
pub use provider_impl::MistralRsProvider;
