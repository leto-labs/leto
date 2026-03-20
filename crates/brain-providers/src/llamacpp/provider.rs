use std::collections::HashMap;
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::sync::Arc;

use async_stream::stream;
use futures::future::BoxFuture;
use tokio::sync::RwLock;

use brain_types::*;
use hf_hub::api::sync::ApiBuilder;
use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{AddBos, LlamaChatMessage, LlamaModel};
use llama_cpp_2::sampling::LlamaSampler;

use super::config::LlamaCppConfig;

/// In-process LLM provider backed by llama.cpp via the `llama-cpp-2` crate.
///
/// Supports multiple models with lazy loading: GGUF files are downloaded from
/// HuggingFace on first use (cached at `~/.cache/huggingface/hub/`) and loaded
/// into memory. Subsequent requests reuse the cached model instance.
pub struct LlamaCppProvider {
    backend: Arc<LlamaBackend>,
    configs: HashMap<String, LlamaCppConfig>,
    models: RwLock<HashMap<String, Arc<LlamaModel>>>,
    default_model: String,
}

impl LlamaCppProvider {
    pub fn new(
        configs: Vec<(String, LlamaCppConfig)>,
        default_model: impl Into<String>,
    ) -> Result<Self, BrainError> {
        let backend = LlamaBackend::init()
            .map_err(|e| BrainError::Inference(format!("failed to init llama backend: {e}")))?;
        Ok(Self {
            backend: Arc::new(backend),
            configs: configs.into_iter().collect(),
            models: RwLock::new(HashMap::new()),
            default_model: default_model.into(),
        })
    }

    pub fn add(mut self, name: impl Into<String>, config: LlamaCppConfig) -> Self {
        self.configs.insert(name.into(), config);
        self
    }

    /// Pre-load a registered model so the first `chat()` call doesn't pay
    /// the download/init cost.
    pub async fn preload(&self, name: &str) -> Result<(), BrainError> {
        self.resolve_model(name).await.map(|_| ())
    }

    async fn resolve_model(&self, name: &str) -> Result<Arc<LlamaModel>, BrainError> {
        {
            let cache = self.models.read().await;
            if let Some(model) = cache.get(name) {
                return Ok(model.clone());
            }
        }

        let config = self
            .configs
            .get(name)
            .ok_or_else(|| BrainError::Inference(format!("unknown model: {name}")))?;

        let model = load_model(&self.backend, config)?;
        let model = Arc::new(model);

        let mut cache = self.models.write().await;
        cache
            .entry(name.to_owned())
            .or_insert_with(|| model.clone());
        Ok(model)
    }
}

/// Download (or find cached) GGUF file from HuggingFace, then load it.
fn load_model(backend: &LlamaBackend, config: &LlamaCppConfig) -> Result<LlamaModel, BrainError> {
    let model_path = download_gguf(&config.model_id, &config.gguf_file)?;

    let mut params = LlamaModelParams::default();
    if let Some(n) = config.n_gpu_layers {
        params = params.with_n_gpu_layers(n);
    }

    LlamaModel::load_from_file(backend, &model_path, &params)
        .map_err(|e| BrainError::Inference(format!("failed to load model: {e}")))
}

/// Download a GGUF file from a HuggingFace repo (or return cached path).
fn download_gguf(model_id: &str, gguf_file: &str) -> Result<PathBuf, BrainError> {
    // If model_id looks like a local path, use it directly
    let path = PathBuf::from(model_id);
    if path.is_dir() {
        return Ok(path.join(gguf_file));
    }

    ApiBuilder::new()
        .with_progress(true)
        .build()
        .map_err(|e| BrainError::Inference(format!("failed to init HF API: {e}")))?
        .model(model_id.to_string())
        .get(gguf_file)
        .map_err(|e| {
            BrainError::Inference(format!("failed to download {model_id}/{gguf_file}: {e}"))
        })
}

fn build_chat_prompt(model: &LlamaModel, messages: &[Message]) -> Result<String, BrainError> {
    let chat_messages: Vec<LlamaChatMessage> = messages
        .iter()
        .map(|m| {
            let role = match m.role {
                Role::System => "system",
                Role::User => "user",
                Role::Assistant => "assistant",
                Role::Tool => "tool",
            };
            LlamaChatMessage::new(role.to_string(), m.content.clone())
                .map_err(|e| BrainError::Inference(format!("invalid chat message: {e}")))
        })
        .collect::<Result<Vec<_>, _>>()?;

    let template = model
        .chat_template(None)
        .map_err(|e| BrainError::Inference(format!("no chat template in model: {e}")))?;

    model
        .apply_chat_template(&template, &chat_messages, true)
        .map_err(|e| BrainError::Inference(format!("failed to apply chat template: {e}")))
}

impl Provider for LlamaCppProvider {
    fn info(&self) -> ProviderInfo {
        let models = self
            .configs
            .keys()
            .map(|name| {
                let leaked_name: &'static str = Box::leak(name.clone().into_boxed_str());
                ModelInfo {
                    id: leaked_name,
                    name: leaked_name,
                    family: None,
                    reasoning: None,
                    tool_call: false,
                    attachment: false,
                    structured_output: None,
                    temperature: None,
                    knowledge: None,
                    release_date: None,
                    last_updated: None,
                    open_weights: None,
                    input_modalities: &["text"],
                    output_modalities: &["text"],
                    cost: None,
                    limit: None,
                    status: None,
                }
            })
            .collect();
        ProviderInfo {
            name: "llamacpp".into(),
            default_model: Some(self.default_model.clone()),
            models,
        }
    }

    fn chat<'a>(
        &'a self,
        messages: &'a [Message],
        _tools: &'a [ToolDef],
        config: &'a InferenceConfig,
        _session_id: Option<ulid::Ulid>,
    ) -> BoxFuture<'a, Result<ChatStream, BrainError>> {
        let model_name = config
            .model
            .as_deref()
            .unwrap_or(&self.default_model)
            .to_owned();

        let messages = messages.to_vec();
        let max_tokens = config.max_tokens.unwrap_or(4096) as usize;
        let temperature = config.temperature.unwrap_or(0.7);

        Box::pin(async move {
            let model = self.resolve_model(&model_name).await?;
            let backend = self.backend.clone();

            let prompt = build_chat_prompt(&model, &messages)?;

            let (tx, mut rx) = tokio::sync::mpsc::channel::<Result<ChatChunk, BrainError>>(32);

            tokio::task::spawn_blocking(move || {
                if let Err(e) =
                    run_inference(&backend, &model, &prompt, max_tokens, temperature, &tx)
                {
                    let _ = tx.blocking_send(Err(e));
                }
            });

            let s = stream! {
                while let Some(chunk) = rx.recv().await {
                    yield chunk;
                }
            };

            Ok(Box::pin(s) as ChatStream)
        })
    }
}

fn run_inference(
    backend: &LlamaBackend,
    model: &LlamaModel,
    prompt: &str,
    max_tokens: usize,
    temperature: f32,
    tx: &tokio::sync::mpsc::Sender<Result<ChatChunk, BrainError>>,
) -> Result<(), BrainError> {
    let tokens = model
        .str_to_token(prompt, AddBos::Always)
        .map_err(|e| BrainError::Inference(format!("tokenization failed: {e}")))?;

    let prompt_len = tokens.len();
    let total_len = prompt_len + max_tokens;

    let ctx_params = LlamaContextParams::default()
        .with_n_ctx(NonZeroU32::new(total_len as u32))
        .with_n_batch(total_len as u32);

    let mut ctx = model
        .new_context(backend, ctx_params)
        .map_err(|e| BrainError::Inference(format!("failed to create context: {e}")))?;

    let mut batch = LlamaBatch::new(total_len, 1);
    let last_idx = tokens.len().saturating_sub(1) as i32;
    for (i, token) in (0_i32..).zip(tokens.iter().copied()) {
        batch
            .add(token, i, &[0], i == last_idx)
            .map_err(|e| BrainError::Inference(format!("batch add failed: {e}")))?;
    }

    ctx.decode(&mut batch)
        .map_err(|e| BrainError::Inference(format!("prompt decode failed: {e}")))?;

    let mut sampler = if temperature <= 0.0 {
        LlamaSampler::greedy()
    } else {
        LlamaSampler::chain_simple([
            LlamaSampler::temp(temperature),
            LlamaSampler::top_p(0.9, 1),
            LlamaSampler::greedy(),
        ])
    };

    let mut decoder = encoding_rs::UTF_8.new_decoder();
    let mut n_cur = tokens.len() as i32;
    let mut completion_tokens = 0u32;

    for _ in 0..max_tokens {
        let token = sampler.sample(&ctx, batch.n_tokens() - 1);
        sampler.accept(token);

        if model.is_eog_token(token) {
            break;
        }

        completion_tokens += 1;

        let piece = model
            .token_to_piece(token, &mut decoder, true, None)
            .map_err(|e| BrainError::Inference(format!("detokenize failed: {e}")))?;

        if !piece.is_empty() {
            if tx
                .blocking_send(Ok(ChatChunk::Delta { content: piece }))
                .is_err()
            {
                return Ok(());
            }
        }

        batch.clear();
        batch
            .add(token, n_cur, &[0], true)
            .map_err(|e| BrainError::Inference(format!("batch add failed: {e}")))?;
        n_cur += 1;

        ctx.decode(&mut batch)
            .map_err(|e| BrainError::Inference(format!("decode failed: {e}")))?;
    }

    let _ = tx.blocking_send(Ok(ChatChunk::Done {
        usage: Some(TokenUsage {
            prompt: prompt_len as u32,
            completion: completion_tokens,
            total: prompt_len as u32 + completion_tokens,
        }),
    }));

    Ok(())
}
