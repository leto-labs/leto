use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use agent_core::{AgentCore, AgentCoreNative, CoreError, CoreEvent, ProviderModelInfo};
use agent_runtime::{BlockDelta, Message, RuntimeEvent, Usage};
use agent_store::{
    InMemoryStore, Project, ProjectConfig, SessionId, SessionUpdate, Store, resolve_project_config,
};
use anyhow::{Context, Result, bail};
use atif::Trajectory;
use futures::StreamExt;
use provider::{MockProvider, Provider};
use provider_openai::{OpenAiApiMode, OpenAiConfigPreset, OpenAiProvider};
use serde::Serialize;
use serde_json::Value;

use crate::commands::ExecCommand;

#[derive(Debug, Serialize)]
struct ExecRunSummary {
    status: &'static str,
    provider: String,
    model: String,
    loop_name: String,
    input_tokens: Option<u32>,
    output_tokens: Option<u32>,
    cache_tokens: Option<u32>,
    cache_write_tokens: Option<u32>,
    reasoning_tokens: Option<u32>,
    cost_usd: Option<f64>,
    iterations: Option<u32>,
    total_tokens: u32,
    trajectory_path: PathBuf,
    events_path: PathBuf,
    error_message: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct ExecUsageSummary {
    input_tokens: Option<u32>,
    output_tokens: Option<u32>,
    cache_read_tokens: Option<u32>,
    cache_write_tokens: Option<u32>,
    reasoning_tokens: Option<u32>,
    total_tokens: u32,
}

#[derive(Debug, Clone)]
struct ExecAtifStarted {
    schema_version: atif::SchemaVersion,
    session_id: String,
    agent: atif::Agent,
}

#[derive(Debug, Clone, Default)]
struct ExecAtifState {
    started: Option<ExecAtifStarted>,
    steps: Vec<atif::Step>,
    completed: Option<Trajectory>,
}

impl ExecAtifState {
    fn observe(&mut self, event: &RuntimeEvent) {
        match event {
            RuntimeEvent::AtifTrajectoryStarted {
                schema_version,
                session_id,
                agent,
            } => {
                self.started = Some(ExecAtifStarted {
                    schema_version: *schema_version,
                    session_id: session_id.clone(),
                    agent: agent.clone(),
                });
            }
            RuntimeEvent::AtifStepCompleted { step } => {
                self.steps.push(step.clone());
            }
            RuntimeEvent::AtifTrajectoryCompleted { trajectory } => {
                self.completed = Some(trajectory.clone());
            }
            _ => {}
        }
    }

    fn build_trajectory(self) -> Result<Trajectory> {
        let started = self
            .started
            .context("exec run did not emit ATIF trajectory start metadata")?;
        let completed = self
            .completed
            .context("exec run did not emit a completed ATIF trajectory")?;

        if completed.schema_version != started.schema_version {
            bail!("completed ATIF trajectory schema version did not match the started event");
        }
        if completed.session_id != started.session_id {
            bail!("completed ATIF trajectory session id did not match the started event");
        }
        if completed.agent != started.agent {
            bail!("completed ATIF trajectory agent metadata did not match the started event");
        }
        if completed.steps != self.steps {
            bail!("completed ATIF trajectory steps did not match the streamed ATIF step events");
        }

        Ok(Trajectory {
            schema_version: started.schema_version,
            session_id: started.session_id,
            agent: started.agent,
            steps: self.steps,
            notes: completed.notes,
            final_metrics: completed.final_metrics,
            continued_trajectory_ref: completed.continued_trajectory_ref,
            extra: completed.extra,
        })
    }
}

/// Runs one non-interactive exec session and writes benchmark artifacts.
pub async fn run_exec(command: ExecCommand) -> Result<()> {
    let cwd = command
        .cwd
        .canonicalize()
        .with_context(|| format!("failed to resolve cwd {}", command.cwd.display()))?;
    let _cwd_guard = WorkingDirectoryGuard::enter(&cwd)?;
    fs::create_dir_all(&command.output_dir)
        .with_context(|| format!("failed to create {}", command.output_dir.display()))?;

    let normalized_model = normalize_model_id(&command.provider, &command.model);
    let normalized_loop = normalize_loop_name(&command.loop_name);

    let store: Arc<dyn Store> = Arc::new(InMemoryStore::new());
    let core = build_exec_core(
        store.clone(),
        &command.provider,
        &normalized_model,
        command.api_key.as_deref(),
        command.base_url.as_deref(),
        command.api_surface.as_deref(),
        &normalized_loop,
    )
    .await
    .context("failed to initialize exec core")?;

    let config = resolve_exec_config(&cwd, &command.provider, &normalized_model, &normalized_loop);
    let project = create_exec_project(store, &cwd, config)
        .await
        .context("failed to create exec project")?;
    let session = core
        .create_session(project.id)
        .await
        .context("failed to create exec session")?;
    core.update_session(
        session.id,
        SessionUpdate {
            provider: Some(Some(command.provider.clone())),
            model: Some(Some(normalized_model.clone())),
            loop_name: Some(Some(normalized_loop.clone())),
            ..SessionUpdate::default()
        },
    )
    .await
    .context("failed to configure exec session")?;

    let events_path = command.output_dir.join("events.jsonl");
    let trajectory_path = command.output_dir.join("trajectory.json");
    let run_path = command.output_dir.join("run.json");

    let mut events_writer = BufWriter::new(
        File::create(&events_path)
            .with_context(|| format!("failed to create {}", events_path.display()))?,
    );

    let mut stream = core
        .turn(session.id, vec![Message::user_text(&command.instruction)])
        .await
        .context("failed to start exec turn")?;
    let mut usage = ExecUsageSummary::default();
    let mut iterations = None;
    let mut error_message = None::<String>;
    let mut saw_turn_done = false;
    let mut atif_state = ExecAtifState::default();

    while let Some(event) = stream.next().await {
        serde_json::to_writer(&mut events_writer, &event).context("failed to serialize event")?;
        events_writer
            .write_all(b"\n")
            .context("failed to write event newline")?;

        match &event {
            CoreEvent::Turn { event, .. } => atif_state.observe(event),
            _ => {}
        }

        match event {
            CoreEvent::Turn {
                event: RuntimeEvent::OutputBlockDelta { delta, .. },
                ..
            } => {
                if let BlockDelta::Text { text } = delta {
                    print!("{text}");
                    std::io::stdout().flush().ok();
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
                event: RuntimeEvent::Usage { usage: turn_usage },
                ..
            } => {
                usage = summarize_usage(turn_usage);
            }
            CoreEvent::Turn {
                event: RuntimeEvent::Error { message, .. },
                ..
            } => {
                eprintln!("\n[error: {message}]");
                error_message = Some(message);
            }
            CoreEvent::Turn {
                event: RuntimeEvent::TurnFinished { turn_index, .. },
                ..
            } => {
                println!();
                saw_turn_done = true;
                iterations = u32::try_from(turn_index)
                    .ok()
                    .and_then(|value| value.checked_add(1));
            }
            CoreEvent::TurnCancelled { .. } => {
                error_message.get_or_insert_with(|| "exec turn was cancelled".to_owned());
            }
            _ => {}
        }
    }

    events_writer.flush().context("failed to flush event log")?;

    let cost_usd = resolve_model_info(&core, session.id, &command.provider)
        .await
        .as_ref()
        .and_then(|model_info| compute_cost_usd(model_info, &usage));
    let mut trajectory = atif_state.build_trajectory()?;
    trajectory = augment_exec_trajectory(trajectory, &cwd, Path::new("events.jsonl"));
    if let Some(final_metrics) = trajectory.final_metrics.as_mut()
        && final_metrics.total_cost_usd.is_none()
    {
        final_metrics.total_cost_usd = cost_usd;
    }
    trajectory
        .validate()
        .map_err(|error| anyhow::anyhow!("invalid ATIF trajectory: {error}"))?;
    write_json(&trajectory_path, &trajectory)?;

    let summary = build_exec_run_summary(
        &command,
        &normalized_model,
        &normalized_loop,
        &usage,
        cost_usd,
        iterations,
        &trajectory_path,
        &events_path,
        error_message.clone(),
        saw_turn_done,
    );
    write_json(&run_path, &summary)?;

    if !saw_turn_done {
        bail!(
            "{}",
            error_message.unwrap_or_else(|| "exec run did not complete normally".to_owned())
        );
    }

    Ok(())
}

async fn build_exec_core(
    store: Arc<dyn Store>,
    provider_name: &str,
    model: &str,
    api_key: Option<&str>,
    base_url: Option<&str>,
    api_surface: Option<&str>,
    loop_name: &str,
) -> Result<AgentCoreNative, CoreError> {
    let provider: Arc<dyn Provider> = if provider_name == "mock" {
        Arc::new(MockProvider::new())
    } else {
        let preset = OpenAiConfigPreset::ALL
            .iter()
            .find(|preset| preset.name == provider_name)
            .ok_or_else(|| CoreError::ProviderNotRegistered(provider_name.to_owned()))?;
        let api_key = api_key.ok_or_else(|| {
            CoreError::Internal(format!(
                "api key is required for exec provider '{provider_name}'"
            ))
        })?;

        let mut config = preset.into_config(api_key).with_model(model);
        if let Some(base_url) = base_url {
            config = config.with_base_url(base_url);
        }
        if let Some(api_surface) = api_surface {
            config = config.with_api_surface_mode(parse_api_surface_mode(api_surface)?);
        }

        Arc::new(OpenAiProvider::new(config))
    };

    AgentCoreNative::builder(store)
        .without_credential_discovery()
        .with_provider(provider_name.to_owned(), provider)
        .default_provider(provider_name.to_owned())
        .default_loop(loop_name.to_owned())
        .build()
        .await
}

fn resolve_exec_config(root: &Path, provider: &str, model: &str, loop_name: &str) -> ProjectConfig {
    let mut config = resolve_project_config(root).unwrap_or_else(|error| {
        tracing::warn!("config resolution failed, using defaults: {error}");
        ProjectConfig::default()
    });
    config.default_provider = Some(provider.to_owned());
    config.default_model = Some(model.to_owned());
    config.default_loop = Some(loop_name.to_owned());
    config.runtime.model = Some(model.to_owned());
    config.runtime.atif.emit_events = true;
    config
}

async fn create_exec_project(
    store: Arc<dyn Store>,
    cwd: &Path,
    config: ProjectConfig,
) -> Result<Project, agent_store::StoreError> {
    let name = cwd
        .file_name()
        .and_then(|segment| segment.to_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| "agent".to_owned());
    store
        .projects()
        .create(Project::new(Some(name), Some(cwd.to_owned()), config))
        .await
}

fn summarize_usage(usage: Usage) -> ExecUsageSummary {
    ExecUsageSummary {
        input_tokens: usage.input_tokens,
        output_tokens: usage.output_tokens,
        cache_read_tokens: usage.cache_read_tokens,
        cache_write_tokens: usage.cache_write_tokens,
        reasoning_tokens: usage.reasoning_tokens,
        total_tokens: usage
            .total_tokens
            .unwrap_or_else(|| usage.input_tokens.unwrap_or(0) + usage.output_tokens.unwrap_or(0)),
    }
}

fn build_exec_run_summary(
    command: &ExecCommand,
    normalized_model: &str,
    normalized_loop: &str,
    usage: &ExecUsageSummary,
    cost_usd: Option<f64>,
    iterations: Option<u32>,
    trajectory_path: &Path,
    events_path: &Path,
    error_message: Option<String>,
    saw_turn_done: bool,
) -> ExecRunSummary {
    ExecRunSummary {
        status: if saw_turn_done {
            "completed"
        } else {
            "internal_error"
        },
        provider: command.provider.clone(),
        model: normalized_model.to_owned(),
        loop_name: normalized_loop.to_owned(),
        input_tokens: usage.input_tokens,
        output_tokens: usage.output_tokens,
        cache_tokens: usage.cache_read_tokens,
        cache_write_tokens: usage.cache_write_tokens,
        reasoning_tokens: usage.reasoning_tokens,
        cost_usd,
        iterations,
        total_tokens: usage.total_tokens,
        trajectory_path: trajectory_path.to_owned(),
        events_path: events_path.to_owned(),
        error_message,
    }
}

async fn resolve_model_info(
    core: &dyn AgentCore,
    session_id: SessionId,
    provider_name: &str,
) -> Option<ProviderModelInfo> {
    let model_id = core
        .current_model_id_for_session(session_id)
        .await
        .ok()
        .flatten()?;
    core.list_models().into_iter().find(|info| {
        info.provider_name == provider_name
            && (info.model.id == model_id
                || normalize_model_id(provider_name, &info.model.id) == model_id)
    })
}

fn normalize_model_id(provider: &str, model: &str) -> String {
    match model.split_once('/') {
        Some((model_provider, model_id)) if model_provider == provider => model_id.to_owned(),
        _ => model.to_owned(),
    }
}

fn normalize_loop_name(loop_name: &str) -> String {
    match loop_name {
        "terminus-kira" => "terminus_kira".to_owned(),
        other => other.to_owned(),
    }
}

fn parse_api_surface_mode(value: &str) -> Result<OpenAiApiMode, CoreError> {
    match value {
        "auto" => Ok(OpenAiApiMode::Auto),
        "responses" => Ok(OpenAiApiMode::Responses),
        "chat-completions" | "chat_completions" => Ok(OpenAiApiMode::ChatCompletions),
        other => Err(CoreError::Internal(format!(
            "unsupported api surface '{other}' (expected auto, responses, or chat-completions)"
        ))),
    }
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<()> {
    let file =
        File::create(path).with_context(|| format!("failed to create {}", path.display()))?;
    let mut writer = BufWriter::new(file);
    serde_json::to_writer_pretty(&mut writer, value)
        .with_context(|| format!("failed to serialize {}", path.display()))?;
    writer
        .write_all(b"\n")
        .with_context(|| format!("failed to finalize {}", path.display()))?;
    writer
        .flush()
        .with_context(|| format!("failed to flush {}", path.display()))?;
    Ok(())
}

fn augment_exec_trajectory(
    mut trajectory: Trajectory,
    project_root: &Path,
    events_path: &Path,
) -> Trajectory {
    let mut extra = trajectory.extra.take().unwrap_or_default();
    extra.insert(
        "project_root".into(),
        Value::String(project_root.display().to_string()),
    );
    extra.insert(
        "events_path".into(),
        Value::String(events_path.display().to_string()),
    );
    trajectory.extra = Some(extra);
    trajectory
}

fn compute_cost_usd(model_info: &ProviderModelInfo, usage: &ExecUsageSummary) -> Option<f64> {
    let pricing = model_info.model.cost.clone()?;
    if usage.input_tokens.is_none()
        && usage.output_tokens.is_none()
        && usage.cache_read_tokens.is_none()
        && usage.cache_write_tokens.is_none()
        && usage.reasoning_tokens.is_none()
    {
        return None;
    }

    let input_tokens = u64::from(usage.input_tokens.unwrap_or(0));
    let output_tokens = u64::from(usage.output_tokens.unwrap_or(0));
    let cache_read_tokens = u64::from(usage.cache_read_tokens.unwrap_or(0));
    let cache_write_tokens = u64::from(usage.cache_write_tokens.unwrap_or(0));
    let reasoning_tokens = u64::from(usage.reasoning_tokens.unwrap_or(0));

    if cache_read_tokens > input_tokens || reasoning_tokens > output_tokens {
        return None;
    }
    if cache_read_tokens > 0 && pricing.cache_read.is_none() {
        return None;
    }
    if cache_write_tokens > 0 && pricing.cache_write.is_none() {
        return None;
    }
    if reasoning_tokens > 0 && pricing.reasoning.is_none() {
        return None;
    }

    let regular_input_tokens = input_tokens - cache_read_tokens;
    let regular_output_tokens = output_tokens - reasoning_tokens;

    Some(
        usd_for_tokens(regular_input_tokens, pricing.input)
            + usd_for_tokens(regular_output_tokens, pricing.output)
            + pricing
                .cache_read
                .map(|rate| usd_for_tokens(cache_read_tokens, rate))
                .unwrap_or(0.0)
            + pricing
                .cache_write
                .map(|rate| usd_for_tokens(cache_write_tokens, rate))
                .unwrap_or(0.0)
            + pricing
                .reasoning
                .map(|rate| usd_for_tokens(reasoning_tokens, rate))
                .unwrap_or(0.0),
    )
}

fn usd_for_tokens(tokens: u64, rate_per_million: f64) -> f64 {
    (tokens as f64 / 1_000_000.0) * rate_per_million
}

struct WorkingDirectoryGuard {
    original: PathBuf,
}

impl WorkingDirectoryGuard {
    fn enter(target: &Path) -> Result<Self> {
        let original = std::env::current_dir().context("failed to capture current directory")?;
        std::env::set_current_dir(target)
            .with_context(|| format!("failed to enter {}", target.display()))?;
        Ok(Self { original })
    }
}

impl Drop for WorkingDirectoryGuard {
    fn drop(&mut self) {
        if let Err(error) = std::env::set_current_dir(&self.original) {
            tracing::warn!(
                "failed to restore working directory {}: {error}",
                self.original.display()
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use ulid::Ulid;

    #[test]
    fn normalize_loop_name_preserves_legacy_kira_alias() {
        assert_eq!(normalize_loop_name("terminus-kira"), "terminus_kira");
        assert_eq!(normalize_loop_name("simple"), "simple");
    }

    #[test]
    fn exec_run_summary_uses_normalized_loop_name() {
        let summary = build_exec_run_summary(
            &ExecCommand {
                cwd: PathBuf::from("/tmp/project"),
                model: "provider/model".into(),
                loop_name: "terminus-kira".into(),
                provider: "openai".into(),
                output_dir: PathBuf::from("/tmp/output"),
                api_key: None,
                base_url: None,
                api_surface: None,
                instruction: "hello".into(),
            },
            "model",
            &normalize_loop_name("terminus-kira"),
            &ExecUsageSummary::default(),
            None,
            None,
            Path::new("trajectory.json"),
            Path::new("events.jsonl"),
            None,
            true,
        );

        assert_eq!(summary.loop_name, "terminus_kira");
    }

    #[test]
    fn parse_api_surface_mode_accepts_known_values() {
        assert!(matches!(
            parse_api_surface_mode("auto").unwrap(),
            OpenAiApiMode::Auto
        ));
        assert!(matches!(
            parse_api_surface_mode("responses").unwrap(),
            OpenAiApiMode::Responses
        ));
        assert!(matches!(
            parse_api_surface_mode("chat-completions").unwrap(),
            OpenAiApiMode::ChatCompletions
        ));
    }

    #[test]
    fn exec_atif_state_builds_trajectory_from_runtime_events() {
        let session_id = Ulid::new().to_string();
        let mut state = ExecAtifState::default();
        let step = atif::Step {
            step_id: 1,
            timestamp: None,
            source: atif::StepSource::User,
            model_name: None,
            reasoning_effort: None,
            message: "hello".into(),
            reasoning_content: None,
            tool_calls: None,
            observation: None,
            metrics: None,
            is_copied_context: None,
            extra: None,
        };

        state.observe(&RuntimeEvent::AtifTrajectoryStarted {
            schema_version: atif::SchemaVersion::V1_6,
            session_id: session_id.clone(),
            agent: atif::Agent {
                name: "agent".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                model_name: Some("mock-model".into()),
                tool_definitions: None,
                extra: None,
            },
        });
        state.observe(&RuntimeEvent::AtifStepCompleted { step: step.clone() });
        state.observe(&RuntimeEvent::AtifTrajectoryCompleted {
            trajectory: Trajectory {
                schema_version: atif::SchemaVersion::V1_6,
                session_id: session_id.clone(),
                agent: atif::Agent {
                    name: "agent".into(),
                    version: env!("CARGO_PKG_VERSION").into(),
                    model_name: Some("mock-model".into()),
                    tool_definitions: None,
                    extra: None,
                },
                final_metrics: Some(atif::FinalMetrics {
                    total_prompt_tokens: Some(1),
                    total_completion_tokens: Some(2),
                    total_cached_tokens: None,
                    total_cost_usd: None,
                    total_steps: Some(1),
                    extra: Some(serde_json::Map::new()),
                }),
                steps: vec![step],
                notes: None,
                continued_trajectory_ref: None,
                extra: None,
            },
        });

        let trajectory = state.build_trajectory().expect("expected trajectory");
        trajectory.validate().expect("trajectory should validate");
        assert_eq!(trajectory.session_id, session_id);
        assert_eq!(trajectory.steps.len(), 1);
        assert_eq!(trajectory.agent.model_name.as_deref(), Some("mock-model"));
    }

    #[test]
    fn exec_atif_state_rejects_step_mismatch_against_completed_payload() {
        let session_id = Ulid::new().to_string();
        let completed = Trajectory {
            schema_version: atif::SchemaVersion::V1_6,
            session_id: session_id.clone(),
            agent: atif::Agent {
                name: "agent".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                model_name: Some("mock-model".into()),
                tool_definitions: None,
                extra: Some(
                    json!({
                        "provider": "mock"
                    })
                    .as_object()
                    .expect("expected object")
                    .clone(),
                ),
            },
            final_metrics: None,
            steps: vec![atif::Step {
                step_id: 1,
                timestamp: None,
                source: atif::StepSource::User,
                model_name: None,
                reasoning_effort: None,
                message: "hello".into(),
                reasoning_content: None,
                tool_calls: None,
                observation: None,
                metrics: None,
                is_copied_context: None,
                extra: None,
            }],
            notes: None,
            continued_trajectory_ref: None,
            extra: None,
        };

        let mut state = ExecAtifState::default();
        state.observe(&RuntimeEvent::AtifTrajectoryStarted {
            schema_version: atif::SchemaVersion::V1_6,
            session_id,
            agent: atif::Agent {
                name: "agent".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                model_name: Some("mock-model".into()),
                tool_definitions: None,
                extra: Some(
                    json!({
                        "provider": "mock"
                    })
                    .as_object()
                    .expect("expected object")
                    .clone(),
                ),
            },
        });
        state.observe(&RuntimeEvent::AtifTrajectoryCompleted {
            trajectory: completed.clone(),
        });

        let error = state
            .build_trajectory()
            .expect_err("expected mismatch error");
        assert!(
            error
                .to_string()
                .contains("completed ATIF trajectory steps did not match")
        );
    }

    #[test]
    fn resolve_exec_config_enables_atif_event_emission() {
        let temp = std::env::temp_dir().join(format!("agent-cli-exec-config-{}", Ulid::new()));
        std::fs::create_dir_all(&temp).unwrap();
        let config = resolve_exec_config(&temp, "mock", "mock-model", "simple");

        assert!(config.runtime.atif.emit_events);
        assert_eq!(config.default_provider.as_deref(), Some("mock"));
        assert_eq!(config.default_model.as_deref(), Some("mock-model"));
        assert_eq!(config.default_loop.as_deref(), Some("simple"));
    }

    #[test]
    fn augment_exec_trajectory_adds_exec_metadata() {
        let trajectory = augment_exec_trajectory(
            Trajectory {
                schema_version: atif::SchemaVersion::V1_6,
                session_id: "session".into(),
                agent: atif::Agent {
                    name: "agent".into(),
                    version: "0.1.0".into(),
                    model_name: None,
                    tool_definitions: None,
                    extra: None,
                },
                final_metrics: None,
                steps: vec![atif::Step {
                    step_id: 1,
                    timestamp: None,
                    source: atif::StepSource::User,
                    model_name: None,
                    reasoning_effort: None,
                    message: "hello".into(),
                    reasoning_content: None,
                    tool_calls: None,
                    observation: None,
                    metrics: None,
                    is_copied_context: None,
                    extra: None,
                }],
                notes: None,
                continued_trajectory_ref: None,
                extra: None,
            },
            Path::new("/tmp/project"),
            Path::new("events.jsonl"),
        );

        let extra = trajectory.extra.expect("expected extra metadata");
        assert_eq!(
            extra.get("project_root").and_then(Value::as_str),
            Some("/tmp/project")
        );
        assert_eq!(
            extra.get("events_path").and_then(Value::as_str),
            Some("events.jsonl")
        );
    }
}
