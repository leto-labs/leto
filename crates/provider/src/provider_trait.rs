//! Core shared provider trait and associated metadata.

use std::pin::Pin;

use futures::Stream;
use futures::future::BoxFuture;
use serde::{Deserialize, Serialize};

use crate::{Error, Event, ModelInfo, ProviderCapabilities, Request};

/// Borrowing event stream returned by [`Provider::stream`].
pub type EventStream<'a> = Pin<Box<dyn Stream<Item = Result<Event, Error>> + Send + 'a>>;

/// Metadata describing a concrete provider implementation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderInfo {
    /// Stable provider name.
    pub name: String,
    /// Default model identifier selected when a request omits `model`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_model: Option<String>,
    /// Shared capability summary for the provider.
    pub capabilities: ProviderCapabilities,
    /// Optional advertised model list.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub models: Vec<ModelInfo>,
}

/// Stream-first provider contract used by the shared SDK.
pub trait Provider: Send + Sync {
    /// Starts streaming inference for a shared [`Request`].
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when the request cannot be mapped to the concrete
    /// provider or when the provider fails to start the stream.
    fn stream<'a>(&'a self, request: &'a Request) -> BoxFuture<'a, Result<EventStream<'a>, Error>>;

    /// Returns static metadata about the provider implementation.
    fn info(&self) -> ProviderInfo;
}
