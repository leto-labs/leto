use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result, bail};
use atif::Trajectory as AtifTrajectory;
use brain_core::{AtifEvent, Event, Project, ProjectConfig, ProviderModelInfo, Session, Store};
use brain_stores::InMemoryStore;
use futures::StreamExt;
use serde::Serialize;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use crate::commands::ExecCommand;
use crate::provider::{ExecRuntimeAuth, build_exec_runtime};

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

pub async fn run_exec(command: ExecCommand) -> Result<()> {
    let cwd = command
        .cwd
        .canonicalize()
        .with_context(|| format!("failed to resolve cwd {}", command.cwd.display()))?;
    let _cwd_guard = WorkingDirectoryGuard::enter(&cwd)?;
    fs::create_dir_all(&command.output_dir)
        .with_context(|| format!("failed to create {}", command.output_dir.display()))?;

    let model = normalize_model_id(&command.provider, &command.model);
    let config = resolve_exec_config(&cwd, &command.provider, &model, &command.loop_name);

    let store: Arc<dyn Store> = Arc::new(InMemoryStore::new());
    let runtime = build_exec_runtime(
        &config,
        store.clone(),
        &ExecRuntimeAuth {
            provider: command.provider.clone(),
            api_key: command.api_key.clone(),
            base_url: command.base_url.clone(),
            api_surface: command.api_surface.clone(),
        },
    )
    .context("failed to initialize exec runtime")?;

    let project = create_exec_project(store.clone(), &cwd, config)
        .await
        .context("failed to create exec project")?;
    let session = create_exec_session(store.clone(), project.id)
        .await
        .context("failed to create exec session")?;

    let events_path = command.output_dir.join("events.jsonl");
    let trajectory_path = command.output_dir.join("trajectory.json");
    let run_path = command.output_dir.join("run.json");

    let mut events_writer = BufWriter::new(
        File::create(&events_path)
            .with_context(|| format!("failed to create {}", events_path.display()))?,
    );

    let mut stream = runtime.turn(session.id, &command.instruction, CancellationToken::new());
    let mut usage = ExecUsageSummary::default();
    let mut iterations = None;
    let mut error_message = None::<String>;
    let mut saw_turn_done = false;
    let mut trajectory = None::<AtifTrajectory>;

    while let Some(event) = stream.next().await {
        serde_json::to_writer(&mut events_writer, &event).context("failed to serialize event")?;
        events_writer
            .write_all(b"\n")
            .context("failed to write event newline")?;

        match event {
            Event::Token { delta } => {
                print!("{delta}");
                std::io::stdout().flush().ok();
            }
            Event::ToolCallStart { name, .. } => {
                eprintln!("\n[tool: {name}]");
            }
            Event::ToolCallDone { result, .. } => {
                let preview: String = result.chars().take(200).collect();
                eprintln!("[result: {preview}]");
            }
            Event::Progress { message, .. } => {
                eprintln!("[progress: {message}]");
            }
            Event::Error { message, .. } => {
                eprintln!("\n[error: {message}]");
                error_message = Some(message);
            }
            Event::Atif {
                event:
                    AtifEvent::TrajectoryCompleted {
                        trajectory: emitted,
                    },
            } => {
                trajectory = Some(augment_exec_trajectory(
                    emitted,
                    &cwd,
                    Path::new("events.jsonl"),
                ));
            }
            Event::TurnDone {
                iterations: turn_iterations,
                prompt_tokens: turn_prompt_tokens,
                completion_tokens: turn_completion_tokens,
                cache_read_tokens: turn_cache_read_tokens,
                cache_write_tokens: turn_cache_write_tokens,
                reasoning_tokens: turn_reasoning_tokens,
                total_tokens: turn_total_tokens,
            } => {
                println!();
                saw_turn_done = true;
                iterations = Some(turn_iterations);
                usage.input_tokens = turn_prompt_tokens;
                usage.output_tokens = turn_completion_tokens;
                usage.cache_read_tokens = turn_cache_read_tokens;
                usage.cache_write_tokens = turn_cache_write_tokens;
                usage.reasoning_tokens = turn_reasoning_tokens;
                usage.total_tokens = turn_total_tokens;
            }
            _ => {}
        }
    }

    events_writer.flush().context("failed to flush event log")?;

    let model_info = runtime.current_model_for_session(session.id).await.ok();
    let cost_usd = model_info
        .as_ref()
        .and_then(|model_info| compute_cost_usd(model_info, &usage));
    let mut trajectory = trajectory.context("exec run did not emit ATIF trajectory")?;
    if let Some(final_metrics) = trajectory.final_metrics.as_mut() {
        if final_metrics.total_cost_usd.is_none() {
            final_metrics.total_cost_usd = cost_usd;
        }
    }
    trajectory
        .validate()
        .map_err(|error| anyhow::anyhow!("invalid ATIF trajectory: {error}"))?;
    write_json(&trajectory_path, &trajectory)?;

    let summary = ExecRunSummary {
        status: if saw_turn_done {
            "completed"
        } else {
            "internal_error"
        },
        provider: command.provider.clone(),
        model,
        loop_name: command.loop_name.clone(),
        input_tokens: usage.input_tokens,
        output_tokens: usage.output_tokens,
        cache_tokens: usage.cache_read_tokens,
        cache_write_tokens: usage.cache_write_tokens,
        reasoning_tokens: usage.reasoning_tokens,
        cost_usd,
        iterations,
        total_tokens: usage.total_tokens,
        trajectory_path: trajectory_path.clone(),
        events_path: events_path.clone(),
        error_message: error_message.clone(),
    };
    write_json(&run_path, &summary)?;

    if !saw_turn_done {
        bail!(
            "{}",
            error_message.unwrap_or_else(|| "exec run did not complete normally".to_owned())
        );
    }

    Ok(())
}

fn resolve_exec_config(root: &Path, provider: &str, model: &str, loop_name: &str) -> ProjectConfig {
    let mut config =
        brain_config::resolve_fs_config_with_global(root, None).unwrap_or_else(|error| {
            tracing::warn!("config resolution failed, using defaults: {error}");
            ProjectConfig::default()
        });

    if config.agent.system_prompt.is_none()
        && let Ok(Some(agents_md)) = brain_config::load_root_agents_md(root)
    {
        config.agent.system_prompt = Some(agents_md);
    }

    config.agent.inference.provider = Some(provider.to_owned());
    config.agent.inference.model = Some(model.to_owned());
    config.agent.loop_name = Some(loop_name.to_owned());
    config.agent.atif.emit_events = true;
    config
}

async fn create_exec_project(
    store: Arc<dyn Store>,
    cwd: &Path,
    config: ProjectConfig,
) -> Result<Project, brain_core::BrainError> {
    let name = cwd
        .file_name()
        .and_then(|segment| segment.to_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| "brain".to_owned());
    let project = Project::new(Some(name), Some(cwd.to_owned()), config);
    let key = project.id;
    store.projects().create(key, project).await
}

async fn create_exec_session(
    store: Arc<dyn Store>,
    project_id: ulid::Ulid,
) -> Result<Session, brain_core::BrainError> {
    let session = Session::new(project_id);
    let key = session.id;
    store.sessions().create(key, session).await
}

fn normalize_model_id(provider: &str, model: &str) -> String {
    match model.split_once('/') {
        Some((model_provider, model_id)) if model_provider == provider => model_id.to_owned(),
        _ => model.to_owned(),
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
    mut trajectory: AtifTrajectory,
    project_root: &Path,
    events_path: &Path,
) -> AtifTrajectory {
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
    let pricing = model_info.model.cost?;
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
                "failed to restore current directory to {}: {error}",
                self.original.display()
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use brain_core::{ModelCost, ModelInfo};
    use tempfile::tempdir;

    #[tokio::test]
    async fn run_exec_writes_artifacts_for_mock_provider() {
        let root = tempdir().unwrap();
        let output_dir = root.path().join("artifacts");

        run_exec(ExecCommand {
            cwd: root.path().to_path_buf(),
            model: "mock-echo".into(),
            loop_name: "simple".into(),
            provider: "mock".into(),
            output_dir: output_dir.clone(),
            api_key: None,
            base_url: None,
            api_surface: None,
            instruction: "hello".into(),
        })
        .await
        .unwrap();

        let run_json = fs::read_to_string(output_dir.join("run.json")).unwrap();
        assert!(run_json.contains("\"status\": \"completed\""));
        assert!(output_dir.join("events.jsonl").exists());
        assert!(output_dir.join("trajectory.json").exists());
    }

    #[test]
    fn normalize_model_id_strips_matching_provider_prefix() {
        assert_eq!(normalize_model_id("openai", "openai/gpt-5.4"), "gpt-5.4");
        assert_eq!(normalize_model_id("openrouter", "gpt-5.4"), "gpt-5.4");
    }

    #[test]
    fn compute_cost_usd_accounts_for_cache_and_reasoning_rates() {
        let usage = ExecUsageSummary {
            input_tokens: Some(100),
            output_tokens: Some(40),
            cache_read_tokens: Some(25),
            cache_write_tokens: Some(5),
            reasoning_tokens: Some(10),
            total_tokens: 140,
        };

        let cost = compute_cost_usd(
            &provider_model_info(Some(ModelCost {
                input: 2.0,
                output: 8.0,
                reasoning: Some(16.0),
                cache_read: Some(0.5),
                cache_write: Some(1.0),
                input_audio: None,
                output_audio: None,
            })),
            &usage,
        )
        .unwrap();

        let expected = usd_for_tokens(75, 2.0)
            + usd_for_tokens(30, 8.0)
            + usd_for_tokens(25, 0.5)
            + usd_for_tokens(5, 1.0)
            + usd_for_tokens(10, 16.0);
        assert!((cost - expected).abs() < f64::EPSILON);
    }

    #[test]
    fn compute_cost_usd_returns_none_when_cache_pricing_is_missing() {
        let usage = ExecUsageSummary {
            input_tokens: Some(100),
            output_tokens: Some(40),
            cache_read_tokens: Some(25),
            cache_write_tokens: None,
            reasoning_tokens: None,
            total_tokens: 140,
        };

        assert!(
            compute_cost_usd(
                &provider_model_info(Some(ModelCost {
                    input: 2.0,
                    output: 8.0,
                    reasoning: None,
                    cache_read: None,
                    cache_write: None,
                    input_audio: None,
                    output_audio: None,
                })),
                &usage,
            )
            .is_none()
        );
    }

    fn provider_model_info(cost: Option<ModelCost>) -> ProviderModelInfo {
        ProviderModelInfo {
            provider: "openai".into(),
            model: ModelInfo {
                id: "gpt-5.4",
                name: "GPT-5.4",
                family: None,
                reasoning: Some(&["low", "medium", "high"]),
                tool_call: true,
                attachment: false,
                structured_output: Some(true),
                temperature: Some(true),
                knowledge: None,
                release_date: None,
                last_updated: None,
                open_weights: None,
                input_modalities: &["text"],
                output_modalities: &["text"],
                cost,
                limit: None,
                status: None,
            },
        }
    }
}
