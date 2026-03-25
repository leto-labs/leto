use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use agent_core::{AgentCore, CoreError, CoreEvent, CoreEventStream, ProviderModelInfo};
use agent_runtime::{Message, RuntimeConfig, RuntimeError};
use agent_store::{
    CredentialEntry, CredentialHealth, CredentialStore, CredentialStoreEvent, CredentialStoreKey,
    MessageStore, MessageStoreEvent, Project, ProjectId, ProjectStore, ProjectStoreEvent, Session,
    SessionId, SessionStore, SessionStoreEvent, SessionUpdate, Store, StoreError, StoreEvent,
    StoreStream, StoredMessage, TrajectoryStore, TrajectoryStoreEvent,
};
use async_stream::stream;
use futures::{Stream, StreamExt, future::BoxFuture};
use provider::{ModelCost, ModelInfo, ModelLimit};
use reqwest::{Method, Url};
use serde::de::DeserializeOwned;
use serde_json::json;

use crate::protocol::{
    AgentServerStatus, CreateSessionRequest, CredentialRecord, ErrorResponse, ProjectRootRequest,
    ProviderModelRecord, SessionRuntimeView, TrajectoryRecord, TurnRequest,
    UpdateCredentialHealthRequest,
};

/// Configuration for connecting `AgentCoreRemote` to an `agent-server`.
#[derive(Debug, Clone)]
pub struct AgentCoreRemoteConfig {
    pub base_url: String,
    pub bearer_token: Option<String>,
    pub timeout: Option<Duration>,
    pub user_agent: Option<String>,
}

impl AgentCoreRemoteConfig {
    /// Creates a new remote-core configuration for one server base URL.
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            bearer_token: None,
            timeout: None,
            user_agent: None,
        }
    }
}

/// Errors raised while establishing or bootstrapping a remote core connection.
#[derive(Debug, thiserror::Error)]
pub enum RemoteError {
    #[error("invalid base url: {0}")]
    InvalidBaseUrl(String),
    #[error("http client build failed: {0}")]
    ClientBuild(String),
    #[error("transport error: {0}")]
    Transport(String),
    #[error("protocol error: {0}")]
    Protocol(String),
    #[error("server error: {0}")]
    Server(String),
}

#[derive(Debug)]
enum WireError {
    Api(ErrorResponse),
    Transport(String),
    Decode(String),
}

#[derive(Clone)]
struct RemoteInner {
    client: reqwest::Client,
    base_url: Url,
}

impl RemoteInner {
    fn url(&self, path: &str) -> Url {
        self.base_url
            .join(path)
            .expect("canonical api path should join successfully")
    }

    fn request(&self, method: Method, path: &str) -> reqwest::RequestBuilder {
        self.client.request(method, self.url(path))
    }

    async fn send(&self, builder: reqwest::RequestBuilder) -> Result<reqwest::Response, WireError> {
        let response = builder
            .send()
            .await
            .map_err(|error| WireError::Transport(error.to_string()))?;
        if response.status().is_success() {
            return Ok(response);
        }

        let status = response.status();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| String::from("failed to read error response body"));
        let error = serde_json::from_str::<ErrorResponse>(&body).unwrap_or_else(|_| {
            ErrorResponse::with_details(
                "http_error",
                format!("request failed with status {status}"),
                json!({ "raw_body": body }),
            )
        });
        Err(WireError::Api(error))
    }

    async fn json<T: DeserializeOwned>(
        &self,
        builder: reqwest::RequestBuilder,
    ) -> Result<T, WireError> {
        let response = self.send(builder).await?;
        response
            .json::<T>()
            .await
            .map_err(|error| WireError::Decode(error.to_string()))
    }

    async fn empty(&self, builder: reqwest::RequestBuilder) -> Result<(), WireError> {
        self.send(builder).await.map(|_| ())
    }

    fn core_events_stream(&self) -> CoreEventStream {
        let inner = self.clone();
        Box::pin(stream! {
            let response = match inner.send(inner.request(Method::GET, "v1/events")).await {
                Ok(response) => response,
                Err(error) => {
                    tracing::warn!("failed to open remote core event stream: {}", describe_wire_error(&error));
                    return;
                }
            };

            let mut bytes = response.bytes_stream();
            let mut buffer = Vec::<u8>::new();
            let mut data_lines = Vec::<String>::new();

            while let Some(result) = bytes.next().await {
                let chunk = match result {
                    Ok(chunk) => chunk,
                    Err(error) => {
                        tracing::warn!("remote core event stream error: {error}");
                        break;
                    }
                };
                buffer.extend_from_slice(&chunk);

                while let Some(position) = buffer.iter().position(|byte| *byte == b'\n') {
                    let mut line = buffer.drain(..=position).collect::<Vec<_>>();
                    if matches!(line.last(), Some(b'\n')) {
                        line.pop();
                    }
                    if matches!(line.last(), Some(b'\r')) {
                        line.pop();
                    }

                    let line = String::from_utf8_lossy(&line);
                    if line.is_empty() {
                        if data_lines.is_empty() {
                            continue;
                        }
                        let data = data_lines.join("\n");
                        data_lines.clear();
                        match serde_json::from_str::<CoreEvent>(&data) {
                            Ok(core_event) => yield core_event,
                            Err(error) => {
                                tracing::warn!("failed to decode core event from SSE: {error}");
                                return;
                            }
                        }
                        continue;
                    }

                    if line.starts_with(':') {
                        continue;
                    }
                    if let Some(data) = line.strip_prefix("data:") {
                        data_lines.push(data.trim_start().to_owned());
                    }
                }
            }

            if !buffer.is_empty() {
                let line = String::from_utf8_lossy(&buffer);
                if !line.starts_with(':') {
                    if let Some(data) = line.strip_prefix("data:") {
                        data_lines.push(data.trim_start().to_owned());
                    }
                }
            }
            if !data_lines.is_empty() {
                let data = data_lines.join("\n");
                match serde_json::from_str::<CoreEvent>(&data) {
                    Ok(core_event) => yield core_event,
                    Err(error) => {
                        tracing::warn!("failed to decode trailing core event from SSE: {error}");
                    }
                }
            }
        })
    }

    fn ndjson_core_event_stream(&self, response: reqwest::Response) -> CoreEventStream {
        Box::pin(
            ndjson_stream::<CoreEvent>(response).filter_map(|event| async move {
                match event {
                    Ok(event) => Some(event),
                    Err(error) => {
                        tracing::warn!("failed to decode remote turn stream: {error}");
                        None
                    }
                }
            }),
        )
    }
}

fn ndjson_stream<T>(
    response: reqwest::Response,
) -> Pin<Box<dyn Stream<Item = Result<T, String>> + Send>>
where
    T: DeserializeOwned + Send + 'static,
{
    Box::pin(stream! {
        let mut bytes = response.bytes_stream();
        let mut buffer = Vec::<u8>::new();

        while let Some(result) = bytes.next().await {
            let chunk = match result {
                Ok(chunk) => chunk,
                Err(error) => {
                    yield Err(error.to_string());
                    return;
                }
            };
            buffer.extend_from_slice(&chunk);

            while let Some(position) = buffer.iter().position(|byte| *byte == b'\n') {
                let line = buffer.drain(..=position).collect::<Vec<_>>();
                let line = &line[..line.len().saturating_sub(1)];
                if line.is_empty() {
                    continue;
                }
                match serde_json::from_slice::<T>(line) {
                    Ok(value) => yield Ok(value),
                    Err(error) => {
                        yield Err(error.to_string());
                        return;
                    }
                }
            }
        }

        if !buffer.is_empty() {
            match serde_json::from_slice::<T>(&buffer) {
                Ok(value) => yield Ok(value),
                Err(error) => yield Err(error.to_string()),
            }
        }
    })
}

fn io_store_error(message: impl Into<String>) -> StoreError {
    StoreError::Io(std::io::Error::other(message.into()))
}

fn describe_wire_error(error: &WireError) -> String {
    match error {
        WireError::Api(error) => error.error.message.clone(),
        WireError::Transport(message) | WireError::Decode(message) => message.clone(),
    }
}

fn parse_store_error(error: ErrorResponse) -> StoreError {
    match error.error.code.as_str() {
        "store_not_found" => StoreError::NotFound(error.error.message),
        "store_already_exists" => StoreError::AlreadyExists(error.error.message),
        "store_invalid_input"
        | "invalid_request"
        | "invalid_project_id"
        | "invalid_session_id"
        | "invalid_message_id" => StoreError::InvalidInput(error.error.message),
        "store_io" | "store_serde" | "http_error" => io_store_error(error.error.message),
        _ => StoreError::InvalidInput(error.error.message),
    }
}

fn parse_core_error(error: ErrorResponse) -> CoreError {
    match error.error.code.as_str() {
        "store_not_found"
        | "store_already_exists"
        | "store_invalid_input"
        | "store_io"
        | "store_serde"
        | "invalid_request"
        | "invalid_project_id"
        | "invalid_session_id"
        | "invalid_message_id"
        | "http_error" => CoreError::Store(parse_store_error(error)),
        "provider_not_registered" => CoreError::ProviderNotRegistered(
            error_detail_string(&error, "name").unwrap_or(error.error.message),
        ),
        "loop_not_registered" => CoreError::LoopNotRegistered(
            error_detail_string(&error, "name").unwrap_or(error.error.message),
        ),
        "turn_active" => error_detail_string(&error, "session_id")
            .and_then(|value| value.parse::<SessionId>().ok())
            .map(CoreError::TurnActive)
            .unwrap_or_else(|| CoreError::Store(io_store_error(error.error.message))),
        "no_providers_registered" => CoreError::NoProvidersRegistered,
        "no_loops_registered" => CoreError::NoLoopsRegistered,
        "runtime_error" => CoreError::Runtime(RuntimeError::Internal(error.error.message)),
        _ => CoreError::Store(io_store_error(error.error.message)),
    }
}

fn error_detail_string(error: &ErrorResponse, key: &str) -> Option<String> {
    error
        .error
        .details
        .as_ref()
        .and_then(|value| value.get(key))
        .and_then(|value| value.as_str())
        .map(str::to_owned)
}

fn wire_to_store_error(error: WireError) -> StoreError {
    match error {
        WireError::Api(error) => parse_store_error(error),
        WireError::Transport(message) | WireError::Decode(message) => io_store_error(message),
    }
}

fn wire_to_core_error(error: WireError) -> CoreError {
    match error {
        WireError::Api(error) => parse_core_error(error),
        WireError::Transport(message) | WireError::Decode(message) => {
            CoreError::Store(io_store_error(message))
        }
    }
}

/// HTTP-backed implementation of the shared `AgentCore` boundary.
pub struct AgentCoreRemote {
    inner: Arc<RemoteInner>,
    store: RemoteStore,
    provider_names: Vec<String>,
    loop_names: Vec<String>,
    default_provider_name: String,
    default_loop_name: String,
    models: Vec<ProviderModelInfo>,
}

impl AgentCoreRemote {
    /// Connects to a remote `agent-server` and caches shared metadata required
    /// by synchronous `AgentCore` methods.
    pub async fn connect(config: AgentCoreRemoteConfig) -> Result<Self, RemoteError> {
        let mut base_url = config.base_url;
        if !base_url.ends_with('/') {
            base_url.push('/');
        }
        let base_url = Url::parse(&base_url)
            .map_err(|error| RemoteError::InvalidBaseUrl(error.to_string()))?;

        let mut builder = reqwest::Client::builder();
        if let Some(timeout) = config.timeout {
            builder = builder.timeout(timeout);
        }
        if let Some(user_agent) = config.user_agent {
            builder = builder.user_agent(user_agent);
        }
        if let Some(token) = config.bearer_token {
            let mut headers = reqwest::header::HeaderMap::new();
            let value = reqwest::header::HeaderValue::from_str(&format!("Bearer {token}"))
                .map_err(|error| RemoteError::ClientBuild(error.to_string()))?;
            headers.insert(reqwest::header::AUTHORIZATION, value);
            builder = builder.default_headers(headers);
        }
        let client = builder
            .build()
            .map_err(|error| RemoteError::ClientBuild(error.to_string()))?;
        let inner = Arc::new(RemoteInner { client, base_url });

        let status: AgentServerStatus = inner
            .json(inner.request(Method::GET, "v1/status"))
            .await
            .map_err(remote_error_from_wire)?;
        let model_records: Vec<ProviderModelRecord> = inner
            .json(inner.request(Method::GET, "v1/models"))
            .await
            .map_err(remote_error_from_wire)?;
        let models: Vec<ProviderModelInfo> = model_records
            .into_iter()
            .map(provider_model_from_record)
            .collect();

        Ok(Self {
            store: RemoteStore::new(inner.clone()),
            inner,
            provider_names: status.provider_names,
            loop_names: status.loop_names,
            default_provider_name: status.default_provider_name,
            default_loop_name: status.default_loop_name,
            models,
        })
    }
}

fn leak_string(value: String) -> &'static str {
    Box::leak(value.into_boxed_str())
}

fn leak_vec(values: Vec<String>) -> &'static [&'static str] {
    let leaked = values
        .into_iter()
        .map(leak_string)
        .collect::<Vec<&'static str>>();
    Box::leak(leaked.into_boxed_slice())
}

fn provider_model_from_record(record: ProviderModelRecord) -> ProviderModelInfo {
    ProviderModelInfo {
        provider_name: record.provider_name,
        model: ModelInfo {
            id: std::borrow::Cow::Borrowed(leak_string(record.model.id)),
            name: std::borrow::Cow::Borrowed(leak_string(record.model.name)),
            family: record
                .model
                .family
                .map(|value| std::borrow::Cow::Borrowed(leak_string(value))),
            reasoning_efforts: std::borrow::Cow::Borrowed(leak_vec(record.model.reasoning_efforts)),
            tool_call: record.model.tool_call,
            attachment: record.model.attachment,
            structured_output: record.model.structured_output,
            temperature: record.model.temperature,
            knowledge: record
                .model
                .knowledge
                .map(|value| std::borrow::Cow::Borrowed(leak_string(value))),
            release_date: record
                .model
                .release_date
                .map(|value| std::borrow::Cow::Borrowed(leak_string(value))),
            last_updated: record
                .model
                .last_updated
                .map(|value| std::borrow::Cow::Borrowed(leak_string(value))),
            open_weights: record.model.open_weights,
            input_modalities: std::borrow::Cow::Borrowed(leak_vec(record.model.input_modalities)),
            output_modalities: std::borrow::Cow::Borrowed(leak_vec(record.model.output_modalities)),
            cost: record.model.cost.map(|cost| ModelCost {
                input: cost.input,
                output: cost.output,
                reasoning: cost.reasoning,
                cache_read: cost.cache_read,
                cache_write: cost.cache_write,
                input_audio: cost.input_audio,
                output_audio: cost.output_audio,
            }),
            limit: record.model.limit.map(|limit| ModelLimit {
                context: limit.context,
                input: limit.input,
                output: limit.output,
            }),
            status: record
                .model
                .status
                .map(|value| std::borrow::Cow::Borrowed(leak_string(value))),
            capabilities: record.model.capabilities,
        },
    }
}

fn remote_error_from_wire(error: WireError) -> RemoteError {
    match error {
        WireError::Api(error) => RemoteError::Server(error.error.message),
        WireError::Transport(message) => RemoteError::Transport(message),
        WireError::Decode(message) => RemoteError::Protocol(message),
    }
}

impl AgentCore for AgentCoreRemote {
    fn store(&self) -> &dyn Store {
        &self.store
    }

    fn provider_names(&self) -> Vec<String> {
        self.provider_names.clone()
    }

    fn loop_names(&self) -> Vec<String> {
        self.loop_names.clone()
    }

    fn default_provider_name(&self) -> &str {
        &self.default_provider_name
    }

    fn default_loop_name(&self) -> &str {
        &self.default_loop_name
    }

    fn subscribe(&self) -> CoreEventStream {
        self.inner.core_events_stream()
    }

    fn resolve_or_create_project(
        &self,
        root: PathBuf,
    ) -> BoxFuture<'_, Result<Project, CoreError>> {
        let inner = self.inner.clone();
        Box::pin(async move {
            inner
                .json(
                    inner
                        .request(Method::POST, "v1/projects/resolve")
                        .json(&ProjectRootRequest { root }),
                )
                .await
                .map_err(wire_to_core_error)
        })
    }

    fn create_session(&self, project_id: ProjectId) -> BoxFuture<'_, Result<Session, CoreError>> {
        let inner = self.inner.clone();
        Box::pin(async move {
            inner
                .json(
                    inner
                        .request(Method::POST, &format!("v1/projects/{project_id}/sessions"))
                        .json(&CreateSessionRequest::default()),
                )
                .await
                .map_err(wire_to_core_error)
        })
    }

    fn project(&self, project_id: ProjectId) -> BoxFuture<'_, Result<Project, CoreError>> {
        let inner = self.inner.clone();
        Box::pin(async move {
            inner
                .json(inner.request(Method::GET, &format!("v1/projects/{project_id}")))
                .await
                .map_err(wire_to_core_error)
        })
    }

    fn session(&self, session_id: SessionId) -> BoxFuture<'_, Result<Session, CoreError>> {
        let inner = self.inner.clone();
        Box::pin(async move {
            inner
                .json(inner.request(Method::GET, &format!("v1/sessions/{session_id}")))
                .await
                .map_err(wire_to_core_error)
        })
    }

    fn sessions_for_project(
        &self,
        project_id: ProjectId,
    ) -> BoxFuture<'_, Result<Vec<Session>, CoreError>> {
        let inner = self.inner.clone();
        Box::pin(async move {
            inner
                .json(inner.request(Method::GET, &format!("v1/projects/{project_id}/sessions")))
                .await
                .map_err(wire_to_core_error)
        })
    }

    fn update_session(
        &self,
        session_id: SessionId,
        update: SessionUpdate,
    ) -> BoxFuture<'_, Result<Session, CoreError>> {
        let inner = self.inner.clone();
        Box::pin(async move {
            inner
                .json(
                    inner
                        .request(Method::PATCH, &format!("v1/sessions/{session_id}"))
                        .json(&update),
                )
                .await
                .map_err(wire_to_core_error)
        })
    }

    fn delete_session(&self, session_id: SessionId) -> BoxFuture<'_, Result<(), CoreError>> {
        let inner = self.inner.clone();
        Box::pin(async move {
            inner
                .empty(inner.request(Method::DELETE, &format!("v1/sessions/{session_id}")))
                .await
                .map_err(wire_to_core_error)
        })
    }

    fn messages(
        &self,
        session_id: SessionId,
    ) -> BoxFuture<'_, Result<Vec<StoredMessage>, CoreError>> {
        let inner = self.inner.clone();
        Box::pin(async move {
            inner
                .json(inner.request(Method::GET, &format!("v1/sessions/{session_id}/messages")))
                .await
                .map_err(wire_to_core_error)
        })
    }

    fn trajectory(
        &self,
        session_id: SessionId,
    ) -> BoxFuture<'_, Result<Option<atif::Trajectory>, CoreError>> {
        let inner = self.inner.clone();
        Box::pin(async move {
            inner
                .json(inner.request(Method::GET, &format!("v1/sessions/{session_id}/trajectory")))
                .await
                .map_err(wire_to_core_error)
        })
    }

    fn upsert_trajectory(
        &self,
        session_id: SessionId,
        trajectory: atif::Trajectory,
    ) -> BoxFuture<'_, Result<atif::Trajectory, CoreError>> {
        let inner = self.inner.clone();
        Box::pin(async move {
            inner
                .json(
                    inner
                        .request(Method::PUT, &format!("v1/sessions/{session_id}/trajectory"))
                        .json(&trajectory),
                )
                .await
                .map_err(wire_to_core_error)
        })
    }

    fn effective_runtime_config(
        &self,
        session_id: SessionId,
    ) -> BoxFuture<'_, Result<RuntimeConfig, CoreError>> {
        let inner = self.inner.clone();
        Box::pin(async move {
            let view: SessionRuntimeView = inner
                .json(inner.request(Method::GET, &format!("v1/sessions/{session_id}/runtime")))
                .await
                .map_err(wire_to_core_error)?;
            Ok(view.config)
        })
    }

    fn current_loop_name_for_session(
        &self,
        session_id: SessionId,
    ) -> BoxFuture<'_, Result<String, CoreError>> {
        let inner = self.inner.clone();
        Box::pin(async move {
            let view: SessionRuntimeView = inner
                .json(inner.request(Method::GET, &format!("v1/sessions/{session_id}/runtime")))
                .await
                .map_err(wire_to_core_error)?;
            Ok(view.current_loop_name)
        })
    }

    fn current_model_id_for_session(
        &self,
        session_id: SessionId,
    ) -> BoxFuture<'_, Result<Option<String>, CoreError>> {
        let inner = self.inner.clone();
        Box::pin(async move {
            let view: SessionRuntimeView = inner
                .json(inner.request(Method::GET, &format!("v1/sessions/{session_id}/runtime")))
                .await
                .map_err(wire_to_core_error)?;
            Ok(view.current_model_id)
        })
    }

    fn list_models(&self) -> Vec<ProviderModelInfo> {
        self.models.clone()
    }

    fn turn(
        &self,
        session_id: SessionId,
        input: Vec<Message>,
    ) -> BoxFuture<'_, Result<CoreEventStream, CoreError>> {
        let inner = self.inner.clone();
        Box::pin(async move {
            let response = inner
                .send(
                    inner
                        .request(Method::POST, &format!("v1/sessions/{session_id}/turns"))
                        .json(&TurnRequest { input }),
                )
                .await
                .map_err(wire_to_core_error)?;
            Ok(inner.ndjson_core_event_stream(response))
        })
    }

    fn cancel_turn(&self, session_id: SessionId) -> BoxFuture<'_, Result<(), CoreError>> {
        let inner = self.inner.clone();
        Box::pin(async move {
            inner
                .empty(inner.request(Method::POST, &format!("v1/sessions/{session_id}/cancel")))
                .await
                .map_err(wire_to_core_error)
        })
    }
}

struct RemoteStore {
    projects: RemoteProjectStore,
    sessions: RemoteSessionStore,
    messages: RemoteMessageStore,
    credentials: RemoteCredentialStore,
    trajectories: RemoteTrajectoryStore,
}

impl RemoteStore {
    fn new(inner: Arc<RemoteInner>) -> Self {
        Self {
            projects: RemoteProjectStore {
                inner: inner.clone(),
            },
            sessions: RemoteSessionStore {
                inner: inner.clone(),
            },
            messages: RemoteMessageStore {
                inner: inner.clone(),
            },
            credentials: RemoteCredentialStore {
                inner: inner.clone(),
            },
            trajectories: RemoteTrajectoryStore { inner },
        }
    }
}

impl Store for RemoteStore {
    fn projects(&self) -> &dyn ProjectStore {
        &self.projects
    }

    fn sessions(&self) -> &dyn SessionStore {
        &self.sessions
    }

    fn messages(&self) -> &dyn MessageStore {
        &self.messages
    }

    fn credentials(&self) -> &dyn CredentialStore {
        &self.credentials
    }

    fn trajectories(&self) -> &dyn TrajectoryStore {
        &self.trajectories
    }

    fn subscribe(&self) -> StoreStream<StoreEvent> {
        Box::pin(
            self.projects
                .inner
                .core_events_stream()
                .filter_map(|event| async move {
                    match event {
                        CoreEvent::Store { event } => Some(event),
                        CoreEvent::Turn { .. } | CoreEvent::TurnCancelled { .. } => None,
                    }
                }),
        )
    }
}

struct RemoteProjectStore {
    inner: Arc<RemoteInner>,
}

impl ProjectStore for RemoteProjectStore {
    fn create(&self, project: Project) -> BoxFuture<'_, Result<Project, StoreError>> {
        let inner = self.inner.clone();
        Box::pin(async move {
            inner
                .json(
                    inner
                        .request(Method::POST, "v1/projects/raw")
                        .json(&project),
                )
                .await
                .map_err(wire_to_store_error)
        })
    }

    fn get(&self, id: ProjectId) -> BoxFuture<'_, Result<Project, StoreError>> {
        let inner = self.inner.clone();
        Box::pin(async move {
            inner
                .json(inner.request(Method::GET, &format!("v1/projects/{id}")))
                .await
                .map_err(wire_to_store_error)
        })
    }

    fn list(&self) -> BoxFuture<'_, Result<Vec<Project>, StoreError>> {
        let inner = self.inner.clone();
        Box::pin(async move {
            inner
                .json(inner.request(Method::GET, "v1/projects"))
                .await
                .map_err(wire_to_store_error)
        })
    }

    fn update(
        &self,
        id: ProjectId,
        project: Project,
    ) -> BoxFuture<'_, Result<Project, StoreError>> {
        let inner = self.inner.clone();
        Box::pin(async move {
            inner
                .json(
                    inner
                        .request(Method::PUT, &format!("v1/projects/{id}"))
                        .json(&project),
                )
                .await
                .map_err(wire_to_store_error)
        })
    }

    fn delete(&self, id: ProjectId) -> BoxFuture<'_, Result<(), StoreError>> {
        let inner = self.inner.clone();
        Box::pin(async move {
            inner
                .empty(inner.request(Method::DELETE, &format!("v1/projects/{id}")))
                .await
                .map_err(wire_to_store_error)
        })
    }

    fn find_by_root(&self, root: &Path) -> BoxFuture<'_, Result<Option<Project>, StoreError>> {
        let inner = self.inner.clone();
        let root = root.to_path_buf();
        Box::pin(async move {
            inner
                .json(
                    inner
                        .request(Method::POST, "v1/projects/find-by-root")
                        .json(&ProjectRootRequest { root }),
                )
                .await
                .map_err(wire_to_store_error)
        })
    }

    fn subscribe(&self) -> StoreStream<ProjectStoreEvent> {
        Box::pin(
            self.inner
                .core_events_stream()
                .filter_map(|event| async move {
                    match event {
                        CoreEvent::Store {
                            event: StoreEvent::Project(event),
                        } => Some(event),
                        CoreEvent::Store { .. }
                        | CoreEvent::Turn { .. }
                        | CoreEvent::TurnCancelled { .. } => None,
                    }
                }),
        )
    }
}

struct RemoteSessionStore {
    inner: Arc<RemoteInner>,
}

impl SessionStore for RemoteSessionStore {
    fn create(&self, session: Session) -> BoxFuture<'_, Result<Session, StoreError>> {
        let inner = self.inner.clone();
        Box::pin(async move {
            inner
                .json(inner.request(Method::POST, "v1/sessions").json(&session))
                .await
                .map_err(wire_to_store_error)
        })
    }

    fn get(&self, id: SessionId) -> BoxFuture<'_, Result<Session, StoreError>> {
        let inner = self.inner.clone();
        Box::pin(async move {
            inner
                .json(inner.request(Method::GET, &format!("v1/sessions/{id}")))
                .await
                .map_err(wire_to_store_error)
        })
    }

    fn list(&self) -> BoxFuture<'_, Result<Vec<Session>, StoreError>> {
        let inner = self.inner.clone();
        Box::pin(async move {
            inner
                .json(inner.request(Method::GET, "v1/sessions"))
                .await
                .map_err(wire_to_store_error)
        })
    }

    fn list_for_project(
        &self,
        project_id: ProjectId,
    ) -> BoxFuture<'_, Result<Vec<Session>, StoreError>> {
        let inner = self.inner.clone();
        Box::pin(async move {
            inner
                .json(inner.request(Method::GET, &format!("v1/projects/{project_id}/sessions")))
                .await
                .map_err(wire_to_store_error)
        })
    }

    fn update(
        &self,
        id: SessionId,
        session: Session,
    ) -> BoxFuture<'_, Result<Session, StoreError>> {
        let inner = self.inner.clone();
        Box::pin(async move {
            inner
                .json(
                    inner
                        .request(Method::PUT, &format!("v1/sessions/{id}"))
                        .json(&session),
                )
                .await
                .map_err(wire_to_store_error)
        })
    }

    fn patch(
        &self,
        id: SessionId,
        update: SessionUpdate,
    ) -> BoxFuture<'_, Result<Session, StoreError>> {
        let inner = self.inner.clone();
        Box::pin(async move {
            inner
                .json(
                    inner
                        .request(Method::PATCH, &format!("v1/sessions/{id}"))
                        .json(&update),
                )
                .await
                .map_err(wire_to_store_error)
        })
    }

    fn delete(&self, id: SessionId) -> BoxFuture<'_, Result<(), StoreError>> {
        let inner = self.inner.clone();
        Box::pin(async move {
            inner
                .empty(inner.request(Method::DELETE, &format!("v1/sessions/{id}")))
                .await
                .map_err(wire_to_store_error)
        })
    }

    fn subscribe(&self) -> StoreStream<SessionStoreEvent> {
        Box::pin(
            self.inner
                .core_events_stream()
                .filter_map(|event| async move {
                    match event {
                        CoreEvent::Store {
                            event: StoreEvent::Session(event),
                        } => Some(event),
                        CoreEvent::Store { .. }
                        | CoreEvent::Turn { .. }
                        | CoreEvent::TurnCancelled { .. } => None,
                    }
                }),
        )
    }
}

struct RemoteMessageStore {
    inner: Arc<RemoteInner>,
}

impl MessageStore for RemoteMessageStore {
    fn list_for_session(
        &self,
        session_id: SessionId,
    ) -> BoxFuture<'_, Result<Vec<StoredMessage>, StoreError>> {
        let inner = self.inner.clone();
        Box::pin(async move {
            inner
                .json(inner.request(Method::GET, &format!("v1/sessions/{session_id}/messages")))
                .await
                .map_err(wire_to_store_error)
        })
    }

    fn replace_for_session(
        &self,
        session_id: SessionId,
        messages: Vec<StoredMessage>,
    ) -> BoxFuture<'_, Result<Vec<StoredMessage>, StoreError>> {
        let inner = self.inner.clone();
        Box::pin(async move {
            inner
                .json(
                    inner
                        .request(Method::PUT, &format!("v1/sessions/{session_id}/messages"))
                        .json(&messages),
                )
                .await
                .map_err(wire_to_store_error)
        })
    }

    fn delete_for_session(&self, session_id: SessionId) -> BoxFuture<'_, Result<(), StoreError>> {
        let inner = self.inner.clone();
        Box::pin(async move {
            inner
                .empty(inner.request(
                    Method::DELETE,
                    &format!("v1/sessions/{session_id}/messages"),
                ))
                .await
                .map_err(wire_to_store_error)
        })
    }

    fn subscribe(&self) -> StoreStream<MessageStoreEvent> {
        Box::pin(
            self.inner
                .core_events_stream()
                .filter_map(|event| async move {
                    match event {
                        CoreEvent::Store {
                            event: StoreEvent::Message(event),
                        } => Some(event),
                        CoreEvent::Store { .. }
                        | CoreEvent::Turn { .. }
                        | CoreEvent::TurnCancelled { .. } => None,
                    }
                }),
        )
    }
}

struct RemoteCredentialStore {
    inner: Arc<RemoteInner>,
}

impl CredentialStore for RemoteCredentialStore {
    fn create(
        &self,
        key: CredentialStoreKey,
        credential: CredentialEntry,
    ) -> BoxFuture<'_, Result<CredentialEntry, StoreError>> {
        let inner = self.inner.clone();
        Box::pin(async move {
            inner
                .json(
                    inner
                        .request(Method::POST, &format!("v1/credentials/{}/{}", key.0, key.1))
                        .json(&credential),
                )
                .await
                .map_err(wire_to_store_error)
        })
    }

    fn get(&self, key: CredentialStoreKey) -> BoxFuture<'_, Result<CredentialEntry, StoreError>> {
        let inner = self.inner.clone();
        Box::pin(async move {
            inner
                .json(inner.request(Method::GET, &format!("v1/credentials/{}/{}", key.0, key.1)))
                .await
                .map_err(wire_to_store_error)
        })
    }

    fn list(
        &self,
    ) -> BoxFuture<'_, Result<Vec<(CredentialStoreKey, CredentialEntry)>, StoreError>> {
        let inner = self.inner.clone();
        Box::pin(async move {
            let records: Vec<CredentialRecord> = inner
                .json(inner.request(Method::GET, "v1/credentials"))
                .await
                .map_err(wire_to_store_error)?;
            Ok(records
                .into_iter()
                .map(|record| {
                    (
                        (record.provider_name, record.credential_id),
                        record.credential,
                    )
                })
                .collect())
        })
    }

    fn list_for_provider(
        &self,
        provider_name: &str,
    ) -> BoxFuture<'_, Result<Vec<(CredentialStoreKey, CredentialEntry)>, StoreError>> {
        let inner = self.inner.clone();
        let provider_name = provider_name.to_owned();
        Box::pin(async move {
            let records: Vec<CredentialRecord> = inner
                .json(inner.request(Method::GET, &format!("v1/credentials/{provider_name}")))
                .await
                .map_err(wire_to_store_error)?;
            Ok(records
                .into_iter()
                .map(|record| {
                    (
                        (record.provider_name, record.credential_id),
                        record.credential,
                    )
                })
                .collect())
        })
    }

    fn update(
        &self,
        key: CredentialStoreKey,
        credential: CredentialEntry,
    ) -> BoxFuture<'_, Result<CredentialEntry, StoreError>> {
        let inner = self.inner.clone();
        Box::pin(async move {
            inner
                .json(
                    inner
                        .request(Method::PUT, &format!("v1/credentials/{}/{}", key.0, key.1))
                        .json(&credential),
                )
                .await
                .map_err(wire_to_store_error)
        })
    }

    fn update_health(
        &self,
        provider_name: &str,
        credential_id: &str,
        health: &CredentialHealth,
    ) -> BoxFuture<'_, Result<(), StoreError>> {
        let inner = self.inner.clone();
        let provider_name = provider_name.to_owned();
        let credential_id = credential_id.to_owned();
        let health = health.clone();
        Box::pin(async move {
            inner
                .empty(
                    inner
                        .request(
                            Method::PATCH,
                            &format!("v1/credentials/{provider_name}/{credential_id}/health"),
                        )
                        .json(&UpdateCredentialHealthRequest { health }),
                )
                .await
                .map_err(wire_to_store_error)
        })
    }

    fn delete(&self, key: CredentialStoreKey) -> BoxFuture<'_, Result<(), StoreError>> {
        let inner = self.inner.clone();
        Box::pin(async move {
            inner
                .empty(inner.request(
                    Method::DELETE,
                    &format!("v1/credentials/{}/{}", key.0, key.1),
                ))
                .await
                .map_err(wire_to_store_error)
        })
    }

    fn subscribe(&self) -> StoreStream<CredentialStoreEvent> {
        Box::pin(
            self.inner
                .core_events_stream()
                .filter_map(|event| async move {
                    match event {
                        CoreEvent::Store {
                            event: StoreEvent::Credential(event),
                        } => Some(event),
                        CoreEvent::Store { .. }
                        | CoreEvent::Turn { .. }
                        | CoreEvent::TurnCancelled { .. } => None,
                    }
                }),
        )
    }
}

struct RemoteTrajectoryStore {
    inner: Arc<RemoteInner>,
}

impl TrajectoryStore for RemoteTrajectoryStore {
    fn get_for_session(
        &self,
        session_id: SessionId,
    ) -> BoxFuture<'_, Result<Option<atif::Trajectory>, StoreError>> {
        let inner = self.inner.clone();
        Box::pin(async move {
            inner
                .json(inner.request(Method::GET, &format!("v1/sessions/{session_id}/trajectory")))
                .await
                .map_err(wire_to_store_error)
        })
    }

    fn list(&self) -> BoxFuture<'_, Result<Vec<(SessionId, atif::Trajectory)>, StoreError>> {
        let inner = self.inner.clone();
        Box::pin(async move {
            let records: Vec<TrajectoryRecord> = inner
                .json(inner.request(Method::GET, "v1/trajectories"))
                .await
                .map_err(wire_to_store_error)?;
            Ok(records
                .into_iter()
                .map(|record| (record.session_id, record.trajectory))
                .collect())
        })
    }

    fn upsert(
        &self,
        session_id: SessionId,
        trajectory: atif::Trajectory,
    ) -> BoxFuture<'_, Result<atif::Trajectory, StoreError>> {
        let inner = self.inner.clone();
        Box::pin(async move {
            inner
                .json(
                    inner
                        .request(Method::PUT, &format!("v1/sessions/{session_id}/trajectory"))
                        .json(&trajectory),
                )
                .await
                .map_err(wire_to_store_error)
        })
    }

    fn delete(&self, session_id: SessionId) -> BoxFuture<'_, Result<(), StoreError>> {
        let inner = self.inner.clone();
        Box::pin(async move {
            inner
                .empty(inner.request(
                    Method::DELETE,
                    &format!("v1/sessions/{session_id}/trajectory"),
                ))
                .await
                .map_err(wire_to_store_error)
        })
    }

    fn subscribe(&self) -> StoreStream<TrajectoryStoreEvent> {
        Box::pin(
            self.inner
                .core_events_stream()
                .filter_map(|event| async move {
                    match event {
                        CoreEvent::Store {
                            event: StoreEvent::Trajectory(event),
                        } => Some(event),
                        CoreEvent::Store { .. }
                        | CoreEvent::Turn { .. }
                        | CoreEvent::TurnCancelled { .. } => None,
                    }
                }),
        )
    }
}
