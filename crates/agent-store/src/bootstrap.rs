use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use dirs::home_dir;
use provider::ReasoningConfig;
use serde::Deserialize;
use toml::Value;

use crate::ProjectConfig;

/// Errors produced while resolving project-local bootstrap configuration.
#[derive(Debug, thiserror::Error)]
pub enum BootstrapError {
    #[error("read config {path}: {source}")]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("parse config {path}: {source}")]
    Parse {
        path: PathBuf,
        source: toml::de::Error,
    },
    #[error("invalid config {path}: {source}")]
    Invalid {
        path: PathBuf,
        source: toml::de::Error,
    },
    #[error("read AGENTS file {path}: {source}")]
    ReadAgents {
        path: PathBuf,
        source: std::io::Error,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
struct RawInferenceConfig {
    provider: Option<String>,
    model: Option<String>,
    reasoning: Option<String>,
    max_tokens: Option<u32>,
    temperature: Option<f64>,
    top_p: Option<f64>,
    top_k: Option<u32>,
    #[serde(flatten)]
    _unknown: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
struct RawAgentConfig {
    max_iterations: Option<u32>,
    system_prompt: Option<String>,
    loop_name: Option<String>,
    tool_output_max_bytes: Option<usize>,
    doom_loop_threshold: Option<u32>,
    compaction_threshold: Option<Option<f32>>,
    compaction_model: Option<String>,
    max_retries: Option<u32>,
    retry_backoff_ms: Option<u64>,
    inference: Option<RawInferenceConfig>,
    #[serde(flatten)]
    _unknown: BTreeMap<String, Value>,
}

/// Returns the global filesystem root used for persisted local CLI state.
///
/// Uses `AGENT_HOME` when present, then `BRAIN_HOME` for compatibility, and
/// otherwise falls back to `~/.brain`.
pub fn agent_home() -> PathBuf {
    if let Some(path) = std::env::var_os("AGENT_HOME") {
        return PathBuf::from(path);
    }
    if let Some(path) = std::env::var_os("BRAIN_HOME") {
        return PathBuf::from(path);
    }
    home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".brain")
}

/// Resolves project-local bootstrap configuration from `.agents/config.toml`
/// plus a root `.agents/AGENTS.md` fallback when no explicit system prompt is
/// configured.
pub fn resolve_project_config(root: &Path) -> Result<ProjectConfig, BootstrapError> {
    let mut config = ProjectConfig::default();

    if let Some(config_path) = find_config_path(root) {
        if let Some(agent) = load_agent_config(&config_path)? {
            apply_agent_config(&mut config, agent);
        }
    }

    if config.system_prompt.is_none()
        && let Some(prompt) = load_root_agents_md(root)?
    {
        config.system_prompt = Some(prompt);
    }

    Ok(config)
}

fn find_config_path(start: &Path) -> Option<PathBuf> {
    let mut cursor = if start.is_file() {
        start.parent().unwrap_or(start).to_owned()
    } else {
        start.to_owned()
    };

    loop {
        let candidate = cursor.join(".agents").join("config.toml");
        if candidate.exists() {
            return Some(candidate);
        }

        let boundary = cursor.join(".git");
        if boundary.exists() && boundary.is_dir() {
            return None;
        }

        match cursor.parent() {
            Some(parent) if parent != cursor => {
                cursor = parent.to_owned();
            }
            _ => return None,
        }
    }
}

fn load_agent_config(path: &Path) -> Result<Option<RawAgentConfig>, BootstrapError> {
    let raw = std::fs::read_to_string(path).map_err(|source| BootstrapError::Read {
        path: path.to_owned(),
        source,
    })?;
    let parsed: Value = toml::from_str(&raw).map_err(|source| BootstrapError::Parse {
        path: path.to_owned(),
        source,
    })?;

    let Some(agent) = parsed.get("agent") else {
        return Ok(None);
    };

    agent
        .clone()
        .try_into::<RawAgentConfig>()
        .map(Some)
        .map_err(|source| BootstrapError::Invalid {
            path: path.to_owned(),
            source,
        })
}

fn load_root_agents_md(root: &Path) -> Result<Option<String>, BootstrapError> {
    let path = root.join(".agents").join("AGENTS.md");
    if !path.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(&path)
        .map_err(|source| BootstrapError::ReadAgents { path, source })?;
    Ok(Some(content))
}

fn apply_agent_config(target: &mut ProjectConfig, source: RawAgentConfig) {
    if let Some(max_iterations) = source.max_iterations {
        target.runtime.max_iterations = max_iterations;
    }
    if let Some(system_prompt) = source.system_prompt {
        target.system_prompt = Some(system_prompt);
    }
    if let Some(loop_name) = source.loop_name {
        target.default_loop = Some(loop_name);
    }
    if let Some(tool_output_max_bytes) = source.tool_output_max_bytes {
        target.runtime.tool_output_max_bytes = tool_output_max_bytes;
    }
    if let Some(doom_loop_threshold) = source.doom_loop_threshold {
        target.runtime.doom_loop.enabled = true;
        target.runtime.doom_loop.threshold = doom_loop_threshold;
    }
    if let Some(compaction_threshold) = source.compaction_threshold {
        target.runtime.compaction.enabled = compaction_threshold.is_some();
        if let Some(threshold) = compaction_threshold {
            target.runtime.compaction.threshold_ratio = threshold;
        }
    }
    if let Some(compaction_model) = source.compaction_model {
        target.runtime.compaction.summary_model = Some(compaction_model);
    }
    if let Some(max_retries) = source.max_retries {
        target.runtime.max_retries = max_retries;
    }
    if let Some(retry_backoff_ms) = source.retry_backoff_ms {
        target.runtime.retry_backoff_ms = retry_backoff_ms;
    }

    if let Some(inference) = source.inference {
        if let Some(provider) = inference.provider {
            target.default_provider = Some(provider);
        }
        if let Some(model) = inference.model {
            target.default_model = Some(model.clone());
            target.runtime.model = Some(model);
        }
        if inference.reasoning.is_some()
            || target.runtime.request.reasoning.is_some()
            || inference.max_tokens.is_some()
            || inference.temperature.is_some()
            || inference.top_p.is_some()
            || inference.top_k.is_some()
        {
            let mut reasoning = target.runtime.request.reasoning.clone().unwrap_or_default();
            if let Some(effort) = inference.reasoning {
                reasoning.effort = Some(effort);
            }
            target.runtime.request.reasoning =
                (!reasoning_is_empty(&reasoning)).then_some(reasoning);
        }
        if let Some(max_tokens) = inference.max_tokens {
            target.runtime.request.max_output_tokens = Some(max_tokens);
        }
        if let Some(temperature) = inference.temperature {
            target.runtime.request.temperature = Some(temperature);
        }
        if let Some(top_p) = inference.top_p {
            target.runtime.request.top_p = Some(top_p);
        }
        if let Some(top_k) = inference.top_k {
            target.runtime.request.top_k = Some(top_k);
        }
    }
}

fn reasoning_is_empty(reasoning: &ReasoningConfig) -> bool {
    reasoning.effort.is_none() && reasoning.summary.is_none() && reasoning.budget_tokens.is_none()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn resolve_project_config_loads_project_local_agent_settings() {
        let dir = tempdir().unwrap();
        let agents_dir = dir.path().join(".agents");
        fs::create_dir_all(&agents_dir).unwrap();
        fs::write(
            agents_dir.join("config.toml"),
            r#"[agent]
max_iterations = 42
loop_name = "planner"
max_retries = 3

[agent.inference]
provider = "mock"
model = "mock-model"
reasoning = "high"
max_tokens = 256
"#,
        )
        .unwrap();

        let config = resolve_project_config(dir.path()).unwrap();
        assert_eq!(config.default_loop.as_deref(), Some("planner"));
        assert_eq!(config.default_provider.as_deref(), Some("mock"));
        assert_eq!(config.default_model.as_deref(), Some("mock-model"));
        assert_eq!(config.runtime.max_iterations, 42);
        assert_eq!(config.runtime.max_retries, 3);
        assert_eq!(config.runtime.request.max_output_tokens, Some(256));
        assert_eq!(
            config
                .runtime
                .request
                .reasoning
                .as_ref()
                .and_then(|reasoning| reasoning.effort.as_deref()),
            Some("high")
        );
    }

    #[test]
    fn resolve_project_config_uses_agents_prompt_as_fallback() {
        let dir = tempdir().unwrap();
        let agents_dir = dir.path().join(".agents");
        fs::create_dir_all(&agents_dir).unwrap();
        fs::write(agents_dir.join("AGENTS.md"), "root instructions").unwrap();

        let config = resolve_project_config(dir.path()).unwrap();
        assert_eq!(config.system_prompt.as_deref(), Some("root instructions"));
    }

    #[test]
    fn resolve_project_config_preserves_explicit_prompt_over_agents() {
        let dir = tempdir().unwrap();
        let agents_dir = dir.path().join(".agents");
        fs::create_dir_all(&agents_dir).unwrap();
        fs::write(
            agents_dir.join("config.toml"),
            "[agent]\nsystem_prompt = \"config prompt\"\n",
        )
        .unwrap();
        fs::write(agents_dir.join("AGENTS.md"), "root instructions").unwrap();

        let config = resolve_project_config(dir.path()).unwrap();
        assert_eq!(config.system_prompt.as_deref(), Some("config prompt"));
    }
}
