use std::{env, fs, path::Path, path::PathBuf};

use brain_types::ProjectConfig;
use serde::Deserialize;
use thiserror::Error;
use toml::Value;

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default)]
struct RawInferenceConfig {
    provider: Option<String>,
    model: Option<String>,
    reasoning: Option<String>,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
    #[serde(flatten)]
    _unknown: std::collections::BTreeMap<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default)]
struct RawAgentConfig {
    max_iterations: Option<u32>,
    system_prompt: Option<String>,
    loop_name: Option<String>,
    tool_output_max_bytes: Option<usize>,
    doom_loop_threshold: Option<u32>,
    doom_loop_strategy: Option<brain_types::DoomLoopStrategy>,
    compaction_threshold: Option<Option<f32>>,
    compaction_model: Option<String>,
    max_retries: Option<u32>,
    inference: Option<RawInferenceConfig>,
    #[serde(flatten)]
    _unknown: std::collections::BTreeMap<String, Value>,
}

impl Default for RawInferenceConfig {
    fn default() -> Self {
        Self {
            provider: None,
            model: None,
            reasoning: None,
            max_tokens: None,
            temperature: None,
            _unknown: std::collections::BTreeMap::new(),
        }
    }
}

impl Default for RawAgentConfig {
    fn default() -> Self {
        Self {
            max_iterations: None,
            system_prompt: None,
            loop_name: None,
            tool_output_max_bytes: None,
            doom_loop_threshold: None,
            doom_loop_strategy: None,
            compaction_threshold: None,
            compaction_model: None,
            max_retries: None,
            inference: None,
            _unknown: std::collections::BTreeMap::new(),
        }
    }
}

impl RawAgentConfig {
    fn apply_to(self, target: &mut ProjectConfig) {
        if let Some(max_iterations) = self.max_iterations {
            target.agent.max_iterations = max_iterations;
        }
        if let Some(prompt) = self.system_prompt {
            target.agent.system_prompt = Some(prompt);
        }
        if let Some(loop_name) = self.loop_name {
            target.agent.loop_name = Some(loop_name);
        }
        if let Some(tool_output_max_bytes) = self.tool_output_max_bytes {
            target.agent.tool_output_max_bytes = tool_output_max_bytes;
        }
        if let Some(doom_loop_threshold) = self.doom_loop_threshold {
            target.agent.doom_loop_threshold = doom_loop_threshold;
        }
        if let Some(doom_loop_strategy) = self.doom_loop_strategy {
            target.agent.doom_loop_strategy = doom_loop_strategy;
        }
        if let Some(compaction_threshold) = self.compaction_threshold {
            target.agent.compaction_threshold = compaction_threshold;
        }
        if let Some(compaction_model) = self.compaction_model {
            target.agent.compaction_model = Some(compaction_model);
        }
        if let Some(max_retries) = self.max_retries {
            target.agent.max_retries = max_retries;
        }
        if let Some(inference) = self.inference {
            if let Some(provider) = inference.provider {
                target.agent.inference.provider = Some(provider);
            }
            if let Some(model) = inference.model {
                target.agent.inference.model = Some(model);
            }
            if let Some(reasoning) = inference.reasoning {
                target.agent.inference.reasoning = Some(reasoning);
            }
            if let Some(max_tokens) = inference.max_tokens {
                target.agent.inference.max_tokens = Some(max_tokens);
            }
            if let Some(temperature) = inference.temperature {
                target.agent.inference.temperature = Some(temperature);
            }
        }
    }
}

fn merge_agent(config: &mut ProjectConfig, source: &RawAgentConfig) {
    source.clone().apply_to(config);
}

#[derive(Debug, Error)]
pub enum ConfigError {
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
    #[error("environment variable not set: {name}")]
    MissingEnvVar { name: String },
    #[error("invalid interpolation syntax in value: {value}")]
    InvalidInterpolation { value: String },
    #[error("path does not exist: {path}")]
    MissingPath { path: PathBuf },
}

pub type Result<T> = std::result::Result<T, ConfigError>;

/// Resolve a project configuration by layering global and project-local configs.
///
/// Precedence (lowest to highest):
/// 1. Defaults
/// 2. `~/.brain/config.toml` (global user preferences)
/// 3. `.agents/config.toml` (project-local, discovered by walking up from `root`)
pub fn resolve_fs_config(root: &Path) -> Result<ProjectConfig> {
    let global_config = brain_stores::brain_home().join("config.toml");
    resolve_fs_config_with_global(root, Some(&global_config))
}

/// Like [`resolve_fs_config`] but with an explicit global config path.
pub fn resolve_fs_config_with_global(
    root: &Path,
    global_config: Option<&Path>,
) -> Result<ProjectConfig> {
    let mut config = ProjectConfig::default();

    if let Some(global) = global_config {
        if global.exists() {
            if let Some(agent_config) = load_project_config(global)? {
                merge_agent(&mut config, &agent_config);
            }
        }
    }

    if let Some(config_path) = find_config_path(root)? {
        if let Some(agent_config) = load_project_config(&config_path)? {
            merge_agent(&mut config, &agent_config);
        }
    }

    Ok(config)
}

fn find_config_path(start: &Path) -> Result<Option<PathBuf>> {
    if !start.exists() {
        return Err(ConfigError::MissingPath {
            path: start.to_owned(),
        });
    }

    let mut cursor = if start.is_file() {
        start.parent().unwrap_or(start).to_owned()
    } else {
        start.to_owned()
    };

    loop {
        let candidate = cursor.join(".agents").join("config.toml");
        if candidate.exists() {
            return Ok(Some(candidate));
        }

        let boundary = cursor.join(".git");
        if boundary.exists() && boundary.is_dir() {
            return Ok(None);
        }

        match cursor.parent() {
            Some(parent) if parent != cursor => {
                cursor = parent.to_owned();
            }
            _ => return Ok(None),
        }
    }
}

fn load_project_config(path: &Path) -> Result<Option<RawAgentConfig>> {
    let raw = fs::read_to_string(path).map_err(|source| ConfigError::Read {
        path: path.to_owned(),
        source,
    })?;

    let mut parsed: Value = toml::from_str(&raw).map_err(|source| ConfigError::Parse {
        path: path.to_owned(),
        source,
    })?;

    interpolate_env_vars(&mut parsed)?;

    let Some(agent) = parsed.get("agent") else {
        return Ok(None);
    };

    let agent = agent
        .clone()
        .try_into::<RawAgentConfig>()
        .map_err(|source| ConfigError::Invalid {
            path: path.to_owned(),
            source,
        })?;
    Ok(Some(agent))
}

fn interpolate_env_vars(value: &mut Value) -> Result<()> {
    match value {
        Value::String(value) => {
            let interpolated = interpolate_string(value)?;
            *value = interpolated;
        }
        Value::Array(values) => {
            for v in values {
                interpolate_env_vars(v)?;
            }
        }
        Value::Table(table) => {
            for (_key, value) in table.iter_mut() {
                interpolate_env_vars(value)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn is_var_name_start(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphabetic()
}

fn is_var_name_char(ch: char) -> bool {
    is_var_name_start(ch) || ch.is_ascii_digit()
}

fn interpolate_string(value: &str) -> Result<String> {
    let chars: Vec<char> = value.chars().collect();
    let mut i = 0usize;
    let mut out = String::with_capacity(value.len());

    while i < chars.len() {
        if chars[i] != '$' {
            out.push(chars[i]);
            i += 1;
            continue;
        }

        if i + 1 >= chars.len() {
            out.push(chars[i]);
            break;
        }

        let next = chars[i + 1];

        if next == '{' {
            let mut j = i + 2;
            while j < chars.len() && chars[j] != '}' {
                j += 1;
            }
            if j >= chars.len() {
                return Err(ConfigError::InvalidInterpolation {
                    value: value.to_owned(),
                });
            }
            let key: String = chars[(i + 2)..j].iter().collect();
            if key.is_empty() {
                out.push_str("${}");
                i = j + 1;
                continue;
            }
            let replacement =
                env::var(&key).map_err(|_| ConfigError::MissingEnvVar { name: key })?;
            out.push_str(&replacement);
            i = j + 1;
            continue;
        }

        if is_var_name_start(next) {
            let mut j = i + 1;
            while j < chars.len() && is_var_name_char(chars[j]) {
                j += 1;
            }
            let key: String = chars[(i + 1)..j].iter().collect();
            let replacement =
                env::var(&key).map_err(|_| ConfigError::MissingEnvVar { name: key })?;
            out.push_str(&replacement);
            i = j;
            continue;
        }

        out.push(chars[i]);
        i += 1;
    }

    Ok(out)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NestedAgents {
    pub path: PathBuf,
    pub content: String,
}

/// Load `<root>/.agents/AGENTS.md` as project-level guidance, if present.
pub fn load_root_agents_md(root: &Path) -> Result<Option<String>> {
    let path = root.join(".agents").join("AGENTS.md");
    if !path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(&path).map_err(|source| ConfigError::Read {
        path: path.clone(),
        source,
    })?;
    Ok(Some(content))
}

/// Collect AGENTS.md files between `project_root` and `cwd` (inclusive), ordered from root to cwd.
pub fn list_nested_agents(project_root: &Path, cwd: &Path) -> Result<Vec<NestedAgents>> {
    let mut prompts = Vec::new();

    if !cwd.starts_with(project_root) {
        return Ok(prompts);
    }

    let mut chain: Vec<PathBuf> = Vec::new();
    for dir in cwd.ancestors() {
        if !dir.starts_with(project_root) {
            break;
        }
        chain.push(dir.to_owned());
    }
    chain.reverse();

    for dir in chain {
        let path = dir.join("AGENTS.md");
        if path.exists() && path.is_file() {
            let content = fs::read_to_string(&path).map_err(|source| ConfigError::Read {
                path: path.clone(),
                source,
            })?;
            prompts.push(NestedAgents { path, content });
        }
    }

    Ok(prompts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn resolve_missing_config_returns_default() {
        let dir = tempdir().unwrap();
        let cfg = resolve_fs_config_with_global(dir.path(), None).unwrap();
        assert_eq!(cfg.agent.max_iterations, 20);
        assert!(cfg.agent.system_prompt.is_none());
        assert!(cfg.agent.inference.model.is_none());
    }

    #[test]
    fn resolve_interpolates_environment_variables() {
        unsafe { env::set_var("BRAIN_CONFIG_TEST_SYSTEM_PROMPT", "hello") };
        let dir = tempdir().unwrap();
        let agents_dir = dir.path().join(".agents");
        fs::create_dir_all(&agents_dir).unwrap();
        fs::write(
            agents_dir.join("config.toml"),
            "[agent]\nsystem_prompt = \"$BRAIN_CONFIG_TEST_SYSTEM_PROMPT\"\n",
        )
        .unwrap();

        let cfg = resolve_fs_config_with_global(dir.path(), None).unwrap();
        assert_eq!(cfg.agent.system_prompt.as_deref(), Some("hello"));
        unsafe { env::remove_var("BRAIN_CONFIG_TEST_SYSTEM_PROMPT") };
    }

    #[test]
    fn resolve_missing_env_reference_errors() {
        let dir = tempdir().unwrap();
        let agents_dir = dir.path().join(".agents");
        fs::create_dir_all(&agents_dir).unwrap();
        fs::write(
            agents_dir.join("config.toml"),
            "[agent]\nsystem_prompt = \"$DOES_NOT_EXIST_VAR\"\n",
        )
        .unwrap();

        let err = resolve_fs_config_with_global(dir.path(), None).unwrap_err();
        match err {
            ConfigError::MissingEnvVar { name } => assert_eq!(name, "DOES_NOT_EXIST_VAR"),
            _ => panic!("unexpected error: {err:?}"),
        }
    }

    #[test]
    fn resolve_stops_at_git_boundary() {
        let root = tempdir().unwrap();
        let project = root.path().join("project");
        let nested = project.join("nested");
        fs::create_dir_all(&nested).unwrap();
        fs::create_dir_all(project.join(".git")).unwrap();
        fs::create_dir_all(root.path().join(".agents")).unwrap();
        fs::write(
            root.path().join(".agents").join("config.toml"),
            "[agent]\nmax_iterations = 50\n",
        )
        .unwrap();

        let cfg = resolve_fs_config_with_global(&nested, None).unwrap();
        assert_eq!(cfg.agent.max_iterations, 20);
    }

    #[test]
    fn load_root_agents_md_reads_file() {
        let dir = tempdir().unwrap();
        let agents_dir = dir.path().join(".agents");
        fs::create_dir_all(&agents_dir).unwrap();
        fs::write(agents_dir.join("AGENTS.md"), "root instructions").unwrap();

        let got = load_root_agents_md(dir.path()).unwrap().unwrap();
        assert!(got.contains("root instructions"));
    }

    #[test]
    fn resolve_merges_global_and_project_config() {
        let global_home = tempdir().unwrap();
        let global_config = global_home.path().join("config.toml");
        fs::write(&global_config, "[agent]\nmax_iterations = 50\n").unwrap();

        let project = tempdir().unwrap();
        let agents_dir = project.path().join(".agents");
        fs::create_dir_all(&agents_dir).unwrap();
        fs::write(
            agents_dir.join("config.toml"),
            "[agent]\nsystem_prompt = \"project prompt\"\n",
        )
        .unwrap();

        let cfg = resolve_fs_config_with_global(project.path(), Some(&global_config)).unwrap();
        assert_eq!(cfg.agent.max_iterations, 50);
        assert_eq!(cfg.agent.system_prompt.as_deref(), Some("project prompt"));
    }

    #[test]
    fn resolve_project_overrides_global() {
        let global_home = tempdir().unwrap();
        let global_config = global_home.path().join("config.toml");
        fs::write(
            &global_config,
            "[agent]\nmax_iterations = 50\nsystem_prompt = \"global\"\n",
        )
        .unwrap();

        let project = tempdir().unwrap();
        let agents_dir = project.path().join(".agents");
        fs::create_dir_all(&agents_dir).unwrap();
        fs::write(
            agents_dir.join("config.toml"),
            "[agent]\nmax_iterations = 10\n",
        )
        .unwrap();

        let cfg = resolve_fs_config_with_global(project.path(), Some(&global_config)).unwrap();
        assert_eq!(cfg.agent.max_iterations, 10);
        assert_eq!(cfg.agent.system_prompt.as_deref(), Some("global"));
    }

    #[test]
    fn list_nested_agents_includes_path_chain() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("AGENTS.md"), "root").unwrap();
        let child = dir.path().join("child");
        let grandchild = child.join("grand");
        fs::create_dir_all(&grandchild).unwrap();
        fs::write(child.join("AGENTS.md"), "child").unwrap();

        let got = list_nested_agents(dir.path(), &grandchild).unwrap();
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].content.trim(), "root");
        assert_eq!(got[1].content.trim(), "child");
    }
}
