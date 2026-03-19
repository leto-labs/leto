use std::cmp::Reverse;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use clap::Parser;
use serde::Deserialize;

const MODELS_DEV_URL: &str = "https://github.com/anomalyco/models.dev";
const MODELS_DEV_BRANCH: &str = "dev";
const MODELS_DEV_PATH: &str = "repocache/anomalyco/models.dev";
const OUTPUT_DIR: &str = "crates/brain-providers/src/openai/presets";

#[derive(Parser, Debug)]
struct Args {
    /// Refresh the local models.dev checkout before generation.
    #[arg(long)]
    refresh: bool,
    /// Verify generated files are up to date without writing.
    #[arg(long)]
    check: bool,
}

#[derive(Clone, Copy)]
struct ProviderSpec {
    module: &'static str,
    const_name: &'static str,
    preset_name: &'static str,
    source_provider_id: &'static str,
    env_key: &'static str,
    base_url: &'static str,
    default_model: &'static str,
}

const PROVIDERS: &[ProviderSpec] = &[
    ProviderSpec {
        module: "openai",
        const_name: "OPENAI",
        preset_name: "openai",
        source_provider_id: "openai",
        env_key: "OPENAI_API_KEY",
        base_url: "https://api.openai.com/v1",
        default_model: "gpt-4o-mini",
    },
    ProviderSpec {
        module: "groq",
        const_name: "GROQ",
        preset_name: "groq",
        source_provider_id: "groq",
        env_key: "GROQ_API_KEY",
        base_url: "https://api.groq.com/openai/v1",
        default_model: "llama-3.3-70b-versatile",
    },
    ProviderSpec {
        module: "deepseek",
        const_name: "DEEPSEEK",
        preset_name: "deepseek",
        source_provider_id: "deepseek",
        env_key: "DEEPSEEK_API_KEY",
        base_url: "https://api.deepseek.com",
        default_model: "deepseek-chat",
    },
    ProviderSpec {
        module: "together",
        const_name: "TOGETHER",
        preset_name: "together",
        source_provider_id: "togetherai",
        env_key: "TOGETHER_API_KEY",
        base_url: "https://api.together.xyz/v1",
        default_model: "meta-llama/Llama-3.3-70B-Instruct-Turbo",
    },
    ProviderSpec {
        module: "xai",
        const_name: "XAI",
        preset_name: "xai",
        source_provider_id: "xai",
        env_key: "XAI_API_KEY",
        base_url: "https://api.x.ai/v1",
        default_model: "grok-3-mini",
    },
    ProviderSpec {
        module: "fireworks",
        const_name: "FIREWORKS",
        preset_name: "fireworks",
        source_provider_id: "fireworks-ai",
        env_key: "FIREWORKS_API_KEY",
        base_url: "https://api.fireworks.ai/inference/v1",
        default_model: "accounts/fireworks/models/llama-v3p3-70b-instruct",
    },
    ProviderSpec {
        module: "mistral",
        const_name: "MISTRAL",
        preset_name: "mistral",
        source_provider_id: "mistral",
        env_key: "MISTRAL_API_KEY",
        base_url: "https://api.mistral.ai/v1",
        default_model: "mistral-large-latest",
    },
    ProviderSpec {
        module: "openrouter",
        const_name: "OPENROUTER",
        preset_name: "openrouter",
        source_provider_id: "openrouter",
        env_key: "OPENROUTER_API_KEY",
        base_url: "https://openrouter.ai/api/v1",
        default_model: "openai/gpt-4o-mini",
    },
    ProviderSpec {
        module: "ollama",
        const_name: "OLLAMA",
        preset_name: "ollama",
        source_provider_id: "ollama-cloud",
        env_key: "OLLAMA_API_KEY",
        base_url: "http://localhost:11434/v1",
        default_model: "llama3.3",
    },
    ProviderSpec {
        module: "gemini",
        const_name: "GEMINI",
        preset_name: "gemini",
        source_provider_id: "google",
        env_key: "GEMINI_API_KEY",
        base_url: "https://generativelanguage.googleapis.com/v1beta/openai",
        default_model: "gemini-2.0-flash",
    },
    ProviderSpec {
        module: "minimax",
        const_name: "MINIMAX",
        preset_name: "minimax",
        source_provider_id: "minimax",
        env_key: "MINIMAX_API_KEY",
        base_url: "https://api.minimax.chat/v1",
        default_model: "MiniMax-Text-01",
    },
];

#[derive(Debug, Deserialize)]
struct CostToml {
    input: Option<f64>,
    output: Option<f64>,
    reasoning: Option<f64>,
    cache_read: Option<f64>,
    cache_write: Option<f64>,
    input_audio: Option<f64>,
    output_audio: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct LimitToml {
    context: Option<u64>,
    input: Option<u64>,
    output: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct ModalitiesToml {
    input: Vec<String>,
    output: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ModelToml {
    name: String,
    family: Option<String>,
    attachment: bool,
    reasoning: bool,
    tool_call: bool,
    structured_output: Option<bool>,
    temperature: Option<bool>,
    knowledge: Option<String>,
    release_date: Option<String>,
    last_updated: Option<String>,
    open_weights: bool,
    #[serde(default)]
    cost: Option<CostToml>,
    #[serde(default)]
    limit: Option<LimitToml>,
    modalities: ModalitiesToml,
    status: Option<String>,
}

struct GeneratedModel {
    id: String,
    data: ModelToml,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let models_dev = ensure_models_dev(args.refresh)?;

    let output_dir = PathBuf::from(OUTPUT_DIR);
    let mut files = Vec::new();
    for provider in PROVIDERS {
        let models = load_models(&models_dev, provider)?;
        files.push((
            format!("{}.rs", provider.module),
            render_provider_module(provider, &models),
        ));
    }
    files.push(("mod.rs".to_owned(), render_mod_rs()));

    write_outputs(&output_dir, &files, args.check)
}

fn ensure_models_dev(refresh: bool) -> Result<PathBuf> {
    let path = PathBuf::from(MODELS_DEV_PATH);
    if path.join(".git").exists() {
        if refresh {
            run_git(&[
                "-C",
                MODELS_DEV_PATH,
                "fetch",
                "origin",
                MODELS_DEV_BRANCH,
                "--depth",
                "1",
            ])?;
            run_git(&["-C", MODELS_DEV_PATH, "reset", "--hard", "FETCH_HEAD"])?;
        }
        return Ok(path);
    }

    let parent = path.parent().context("models.dev path has no parent")?;
    fs::create_dir_all(parent)?;
    run_git(&[
        "clone",
        "--depth",
        "1",
        "--branch",
        MODELS_DEV_BRANCH,
        MODELS_DEV_URL,
        MODELS_DEV_PATH,
    ])?;
    Ok(path)
}

fn run_git(args: &[&str]) -> Result<()> {
    let status = Command::new("git").args(args).status()?;
    if !status.success() {
        bail!("git {} failed with status {status}", args.join(" "));
    }
    Ok(())
}

fn load_models(models_dev: &Path, provider: &ProviderSpec) -> Result<Vec<GeneratedModel>> {
    let models_dir = models_dev
        .join("providers")
        .join(provider.source_provider_id)
        .join("models");
    let mut model_files = Vec::new();
    collect_model_files(&models_dir, &mut model_files)?;

    let mut models = Vec::new();
    for file in model_files {
        let relative = file
            .strip_prefix(&models_dir)
            .with_context(|| format!("failed to strip prefix from {}", file.display()))?;
        let model_id = relative_model_id(relative)?;
        let raw = fs::read_to_string(&file)?;
        let model: ModelToml =
            toml::from_str(&raw).with_context(|| format!("parse {}", file.display()))?;
        if should_emit_model(provider, &model_id, &model) {
            models.push(GeneratedModel {
                id: model_id,
                data: model,
            });
        }
    }
    models.sort_by(|left, right| {
        Reverse(model_sort_date(&left.data))
            .cmp(&Reverse(model_sort_date(&right.data)))
            .then_with(|| left.id.cmp(&right.id))
    });
    Ok(models)
}

fn model_sort_date(model: &ModelToml) -> &str {
    model
        .release_date
        .as_deref()
        .or(model.last_updated.as_deref())
        .unwrap_or("")
}

fn collect_model_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_model_files(&path, files)?;
        } else if path.extension() == Some(OsStr::new("toml")) {
            files.push(path);
        }
    }
    Ok(())
}

fn relative_model_id(path: &Path) -> Result<String> {
    let mut parts = path
        .iter()
        .map(|part| part.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    let last = parts.last_mut().context("empty model path")?;
    *last = last
        .strip_suffix(".toml")
        .context("model file missing .toml suffix")?
        .to_owned();
    Ok(parts.join("/"))
}

fn should_emit_model(provider: &ProviderSpec, model_id: &str, model: &ModelToml) -> bool {
    if !model
        .modalities
        .output
        .iter()
        .any(|modality| modality == "text")
    {
        return false;
    }

    if model.status.as_deref() == Some("deprecated") {
        return false;
    }

    if is_non_chat_model(model_id, &model.name, model.family.as_deref()) {
        return false;
    }

    let openai_codex = provider.preset_name == "openai"
        && (model
            .family
            .as_deref()
            .unwrap_or_default()
            .contains("codex")
            || model_id.contains("codex")
            || model.name.to_ascii_lowercase().contains("codex"));
    if openai_codex {
        return false;
    }

    true
}

fn is_non_chat_model(model_id: &str, name: &str, family: Option<&str>) -> bool {
    let id = model_id.to_ascii_lowercase();
    let display_name = name.to_ascii_lowercase();
    let family = family.unwrap_or_default().to_ascii_lowercase();

    const NON_CHAT_KEYWORDS: &[&str] = &[
        "embedding",
        "rerank",
        "moderation",
        "transcription",
        "transcribe",
        "speech",
        "tts",
    ];

    NON_CHAT_KEYWORDS.iter().any(|keyword| {
        id.contains(keyword) || display_name.contains(keyword) || family.contains(keyword)
    })
}

fn render_provider_module(provider: &ProviderSpec, models: &[GeneratedModel]) -> String {
    let mut out = String::new();
    out.push_str("// This file is generated by `provider-preset-gen`. Do not edit manually.\n");
    out.push_str("use brain_types::ModelInfo;\n\n");
    out.push_str("use super::{OpenAiConfigPreset, REASONING_LMH};\n\n");
    out.push_str("pub const MODELS: &[ModelInfo] = &[\n");
    for model in models {
        out.push_str(&render_model(model));
    }
    out.push_str("];\n\n");
    out.push_str("pub const PRESET: OpenAiConfigPreset = OpenAiConfigPreset {\n");
    out.push_str(&format!("    name: {:?},\n", provider.preset_name));
    out.push_str(&format!("    base_url: {:?},\n", provider.base_url));
    out.push_str(&format!(
        "    default_model: {:?},\n",
        provider.default_model
    ));
    out.push_str(&format!("    env_key: {:?},\n", provider.env_key));
    out.push_str("    models: MODELS,\n");
    out.push_str("};\n");
    out
}

fn render_model(model: &GeneratedModel) -> String {
    let m = &model.data;
    let mut out = String::new();
    out.push_str("    ModelInfo {\n");
    out.push_str(&format!("        id: {:?},\n", model.id));
    out.push_str(&format!("        name: {:?},\n", m.name));
    out.push_str(&format!(
        "        family: {},\n",
        render_opt_str(m.family.as_deref())
    ));
    out.push_str(&format!(
        "        reasoning: {},\n",
        if m.reasoning {
            "Some(REASONING_LMH)"
        } else {
            "None"
        }
    ));
    out.push_str(&format!("        tool_call: {},\n", m.tool_call));
    out.push_str(&format!("        attachment: {},\n", m.attachment));
    out.push_str(&format!(
        "        structured_output: {},\n",
        render_opt_bool(m.structured_output)
    ));
    out.push_str(&format!(
        "        temperature: {},\n",
        render_opt_bool(m.temperature)
    ));
    out.push_str(&format!(
        "        knowledge: {},\n",
        render_opt_str(m.knowledge.as_deref())
    ));
    out.push_str(&format!(
        "        release_date: {},\n",
        render_opt_str(m.release_date.as_deref())
    ));
    out.push_str(&format!(
        "        last_updated: {},\n",
        render_opt_str(m.last_updated.as_deref())
    ));
    out.push_str(&format!(
        "        open_weights: Some({}),\n",
        if m.open_weights { "true" } else { "false" }
    ));
    out.push_str(&format!(
        "        input_modalities: {},\n",
        render_str_slice(&m.modalities.input)
    ));
    out.push_str(&format!(
        "        output_modalities: {},\n",
        render_str_slice(&m.modalities.output)
    ));
    out.push_str(&format!(
        "        cost: {},\n",
        render_cost(m.cost.as_ref())
    ));
    out.push_str(&format!(
        "        limit: {},\n",
        render_limit(m.limit.as_ref())
    ));
    out.push_str(&format!(
        "        status: {},\n",
        render_opt_str(m.status.as_deref())
    ));
    out.push_str("    },\n");
    out
}

fn render_opt_str(value: Option<&str>) -> String {
    match value {
        Some(value) => format!("Some({value:?})"),
        None => "None".to_owned(),
    }
}

fn render_opt_bool(value: Option<bool>) -> String {
    match value {
        Some(true) => "Some(true)".to_owned(),
        Some(false) => "Some(false)".to_owned(),
        None => "None".to_owned(),
    }
}

fn render_str_slice(values: &[String]) -> String {
    let items = values
        .iter()
        .map(|value| format!("{value:?}"))
        .collect::<Vec<_>>()
        .join(", ");
    format!("&[{items}]")
}

fn render_cost(cost: Option<&CostToml>) -> String {
    match cost {
        Some(cost) => format!(
            "Some(brain_types::ModelCost {{ input: {}, output: {}, reasoning: {}, cache_read: {}, cache_write: {}, input_audio: {}, output_audio: {} }})",
            render_f64(cost.input),
            render_f64(cost.output),
            render_opt_f64(cost.reasoning),
            render_opt_f64(cost.cache_read),
            render_opt_f64(cost.cache_write),
            render_opt_f64(cost.input_audio),
            render_opt_f64(cost.output_audio),
        ),
        None => "None".to_owned(),
    }
}

fn render_limit(limit: Option<&LimitToml>) -> String {
    match limit {
        Some(limit) => format!(
            "Some(brain_types::ModelLimit {{ context: {}, input: {}, output: {} }})",
            limit.context.unwrap_or(0),
            render_opt_u64(limit.input),
            limit.output.unwrap_or(0),
        ),
        None => "None".to_owned(),
    }
}

fn render_opt_f64(value: Option<f64>) -> String {
    match value {
        Some(value) => format!("Some({})", format_number(value)),
        None => "None".to_owned(),
    }
}

fn render_f64(value: Option<f64>) -> String {
    format_number(value.unwrap_or(0.0))
}

fn format_number(value: f64) -> String {
    let mut rendered = format!("{value:.6}");
    while rendered.contains('.') && rendered.ends_with('0') {
        rendered.pop();
    }
    if rendered.ends_with('.') {
        rendered.push('0');
    }
    rendered
}

fn render_opt_u64(value: Option<u64>) -> String {
    match value {
        Some(value) => format!("Some({value})"),
        None => "None".to_owned(),
    }
}

fn render_mod_rs() -> String {
    let mut out = String::new();
    out.push_str("// This file is generated by `provider-preset-gen`. Do not edit manually.\n");
    for provider in PROVIDERS {
        out.push_str(&format!("pub mod {};\n", provider.module));
    }
    out.push_str("\nuse brain_types::ModelInfo;\n\n");
    out.push_str("use super::config::OpenAiConfig;\n\n");
    out.push_str("const REASONING_LMH: &[&str] = &[\"low\", \"medium\", \"high\"];\n\n");
    out.push_str("#[derive(Debug, Clone, Copy)]\n");
    out.push_str("pub struct OpenAiConfigPreset {\n");
    out.push_str("    pub name: &'static str,\n");
    out.push_str("    pub base_url: &'static str,\n");
    out.push_str("    pub default_model: &'static str,\n");
    out.push_str("    pub env_key: &'static str,\n");
    out.push_str("    pub models: &'static [ModelInfo],\n");
    out.push_str("}\n\n");
    out.push_str("impl OpenAiConfigPreset {\n");
    for provider in PROVIDERS {
        out.push_str(&format!(
            "    pub const {}: Self = {}::PRESET;\n",
            provider.const_name, provider.module
        ));
    }
    out.push_str("\n    pub const ALL: &[Self] = &[\n");
    for provider in PROVIDERS {
        out.push_str(&format!("        Self::{},\n", provider.const_name));
    }
    out.push_str("    ];\n\n");
    out.push_str("    pub fn by_name(name: &str) -> Option<Self> {\n");
    out.push_str("        let lower = name.to_ascii_lowercase();\n");
    out.push_str("        Self::ALL.iter().find(|preset| preset.name == lower).copied()\n");
    out.push_str("    }\n\n");
    out.push_str("    pub fn into_config(self, api_key: impl Into<String>) -> OpenAiConfig {\n");
    out.push_str("        OpenAiConfig {\n");
    out.push_str("            name: self.name.to_owned(),\n");
    out.push_str("            api_key: api_key.into(),\n");
    out.push_str("            base_url: self.base_url.to_owned(),\n");
    out.push_str("            default_model: self.default_model.to_owned(),\n");
    out.push_str("            models: self.models,\n");
    out.push_str("        }\n");
    out.push_str("    }\n\n");
    out.push_str("    pub fn from_env(self) -> Option<OpenAiConfig> {\n");
    out.push_str("        std::env::var(self.env_key).ok().map(|key| self.into_config(key))\n");
    out.push_str("    }\n");
    out.push_str("}\n");
    out
}

fn write_outputs(output_dir: &Path, files: &[(String, String)], check: bool) -> Result<()> {
    if !check {
        fs::create_dir_all(output_dir)?;
    }

    for (name, content) in files {
        let path = output_dir.join(name);
        if check {
            let existing = fs::read_to_string(&path)
                .with_context(|| format!("missing generated file {}", path.display()))?;
            if existing != *content {
                bail!("generated file out of date: {}", path.display());
            }
        } else {
            fs::write(&path, content)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_model_id_preserves_nested_paths() {
        let path = Path::new("openai/gpt-oss-120b.toml");
        let model_id = relative_model_id(path).unwrap();
        assert_eq!(model_id, "openai/gpt-oss-120b");
    }

    #[test]
    fn openai_filter_excludes_codex_models() {
        let provider = PROVIDERS
            .iter()
            .find(|provider| provider.preset_name == "openai")
            .unwrap();
        let model = ModelToml {
            name: "GPT-5.3 Codex".into(),
            family: None,
            attachment: true,
            reasoning: true,
            tool_call: true,
            structured_output: Some(true),
            temperature: Some(false),
            knowledge: None,
            release_date: Some("2026-02-05".into()),
            last_updated: Some("2026-02-05".into()),
            open_weights: false,
            cost: None,
            limit: None,
            modalities: ModalitiesToml {
                input: vec!["text".into()],
                output: vec!["text".into()],
            },
            status: None,
        };

        assert!(!should_emit_model(provider, "gpt-5.3-codex", &model));
        let non_codex = ModelToml {
            name: "GPT-5.4".into(),
            family: Some("gpt".into()),
            ..model
        };
        assert!(should_emit_model(provider, "gpt-5.4", &non_codex));
    }

    #[test]
    fn filter_excludes_embedding_models() {
        let provider = &PROVIDERS[0];
        let model = ModelToml {
            name: "text-embedding-3-small".into(),
            family: Some("text-embedding".into()),
            attachment: false,
            reasoning: false,
            tool_call: false,
            structured_output: None,
            temperature: Some(false),
            knowledge: None,
            release_date: None,
            last_updated: None,
            open_weights: false,
            cost: None,
            limit: None,
            modalities: ModalitiesToml {
                input: vec!["text".into()],
                output: vec!["text".into()],
            },
            status: None,
        };

        assert!(!should_emit_model(
            provider,
            "text-embedding-3-small",
            &model
        ));
    }

    #[test]
    fn newer_release_dates_sort_first() {
        let mut models = vec![
            GeneratedModel {
                id: "older".into(),
                data: ModelToml {
                    name: "Older".into(),
                    family: Some("gpt".into()),
                    attachment: false,
                    reasoning: false,
                    tool_call: true,
                    structured_output: None,
                    temperature: Some(true),
                    knowledge: None,
                    release_date: Some("2025-01-01".into()),
                    last_updated: Some("2025-01-01".into()),
                    open_weights: false,
                    cost: None,
                    limit: None,
                    modalities: ModalitiesToml {
                        input: vec!["text".into()],
                        output: vec!["text".into()],
                    },
                    status: None,
                },
            },
            GeneratedModel {
                id: "newer".into(),
                data: ModelToml {
                    name: "Newer".into(),
                    family: Some("gpt".into()),
                    attachment: false,
                    reasoning: false,
                    tool_call: true,
                    structured_output: None,
                    temperature: Some(true),
                    knowledge: None,
                    release_date: Some("2026-02-01".into()),
                    last_updated: Some("2026-02-01".into()),
                    open_weights: false,
                    cost: None,
                    limit: None,
                    modalities: ModalitiesToml {
                        input: vec!["text".into()],
                        output: vec!["text".into()],
                    },
                    status: None,
                },
            },
        ];

        models.sort_by(|left, right| {
            Reverse(model_sort_date(&left.data))
                .cmp(&Reverse(model_sort_date(&right.data)))
                .then_with(|| left.id.cmp(&right.id))
        });

        assert_eq!(models[0].id, "newer");
        assert_eq!(models[1].id, "older");
    }
}
