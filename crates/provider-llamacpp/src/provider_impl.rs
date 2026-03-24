use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::sync::Arc;

use async_stream::stream;
use base64::Engine;
use futures::future::BoxFuture;
use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::{AddBos, LlamaChatMessage, LlamaModel};
use llama_cpp_2::mtmd::{MtmdBitmap, MtmdContext, MtmdContextParams, MtmdInputText};
use llama_cpp_2::openai::OpenAIChatTemplateParams;
use llama_cpp_2::sampling::LlamaSampler;
use provider::{
    Block, BlockDelta, BlockKind, Error, Event, EventStream, FinishReason, MessageRole, ModelInfo,
    Provider, ProviderCapabilities, ProviderInfo, Request,
};
use tokio::sync::RwLock;

use crate::config::LlamaCppConfig;

struct LoadedModel {
    model: Arc<LlamaModel>,
    mtmd: Option<Arc<MtmdContext>>,
}

/// Shared `provider::Provider` adapter backed by local llama.cpp inference.
pub struct LlamaCppProvider {
    backend: Arc<LlamaBackend>,
    configs: HashMap<String, LlamaCppConfig>,
    models: RwLock<HashMap<String, Arc<LoadedModel>>>,
    default_model: String,
}

impl LlamaCppProvider {
    /// Creates a provider from named configs and a default model identifier.
    pub fn new(
        configs: Vec<(String, LlamaCppConfig)>,
        default_model: impl Into<String>,
    ) -> Result<Self, Error> {
        let backend = LlamaBackend::init()
            .map_err(|err| Error::Inference(format!("failed to init llama backend: {err}")))?;
        Ok(Self {
            backend: Arc::new(backend),
            configs: configs.into_iter().collect(),
            models: RwLock::new(HashMap::new()),
            default_model: default_model.into(),
        })
    }

    /// Adds another named model config to the provider registry.
    pub fn add(mut self, name: impl Into<String>, config: LlamaCppConfig) -> Self {
        self.configs.insert(name.into(), config);
        self
    }

    /// Resolves and caches a named model without issuing an inference request.
    pub async fn preload(&self, name: &str) -> Result<(), Error> {
        self.resolve_model(name).await.map(|_| ())
    }

    async fn resolve_model(&self, name: &str) -> Result<Arc<LoadedModel>, Error> {
        {
            let cache = self.models.read().await;
            if let Some(model) = cache.get(name) {
                return Ok(model.clone());
            }
        }

        let config = self
            .configs
            .get(name)
            .ok_or_else(|| Error::Configuration(format!("unknown model: {name}")))?;
        let loaded = Arc::new(load_model(&self.backend, config)?);

        let mut cache = self.models.write().await;
        Ok(cache
            .entry(name.to_owned())
            .or_insert_with(|| loaded.clone())
            .clone())
    }
}

fn load_model(backend: &LlamaBackend, config: &LlamaCppConfig) -> Result<LoadedModel, Error> {
    let model_path = resolve_file(&config.model_id, &config.gguf_file)?;

    let mut params = llama_cpp_2::model::params::LlamaModelParams::default();
    if let Some(n) = config.n_gpu_layers {
        params = params.with_n_gpu_layers(n);
    }

    let model = LlamaModel::load_from_file(backend, &model_path, &params)
        .map_err(|err| Error::Inference(format!("failed to load model: {err}")))?;
    let model = Arc::new(model);

    let mtmd = match &config.mmproj_file {
        Some(mmproj_file) => {
            let mmproj_path = resolve_file(&config.model_id, mmproj_file)?;
            let params = MtmdContextParams::default();
            Some(Arc::new(
                MtmdContext::init_from_file(
                    mmproj_path.to_string_lossy().as_ref(),
                    &model,
                    &params,
                )
                .map_err(|err| Error::Inference(format!("failed to init mtmd context: {err}")))?,
            ))
        }
        None => None,
    };

    Ok(LoadedModel { model, mtmd })
}

impl Provider for LlamaCppProvider {
    fn stream<'a>(&'a self, request: &'a Request) -> BoxFuture<'a, Result<EventStream<'a>, Error>> {
        Box::pin(async move {
            let model_name = request
                .model
                .clone()
                .unwrap_or_else(|| self.default_model.clone());
            let loaded = self.resolve_model(&model_name).await?;
            let plan = build_inference_plan(&loaded, request).await?;
            let backend = self.backend.clone();

            let (tx, mut rx) = tokio::sync::mpsc::channel::<Result<Event, Error>>(64);
            tokio::task::spawn_blocking(move || {
                let _ = tx.blocking_send(Ok(Event::ResponseStart {
                    response_id: None,
                    model: Some(model_name),
                }));
                if let Err(err) = run_inference(&backend, &loaded, plan, &tx) {
                    let _ = tx.blocking_send(Err(err));
                }
            });

            let output = stream! {
                while let Some(event) = rx.recv().await {
                    yield event;
                }
            };

            Ok(Box::pin(output) as EventStream<'a>)
        })
    }

    fn info(&self) -> ProviderInfo {
        let models = self
            .configs
            .iter()
            .map(|(name, config)| {
                if let Some(model) = &config.model_info {
                    let mut model = model.clone();
                    if model.capabilities.is_none() {
                        model.capabilities = config.capabilities.clone();
                    }
                    model
                } else {
                    ModelInfo {
                        id: Cow::Owned(name.clone()),
                        name: Cow::Owned(name.clone()),
                        family: None,
                        reasoning_efforts: Cow::Borrowed(&[]),
                        tool_call: config
                            .capabilities
                            .as_ref()
                            .map(|capabilities| capabilities.tool_calls)
                            .unwrap_or(false),
                        attachment: config.mmproj_file.is_some(),
                        structured_output: None,
                        temperature: Some(true),
                        knowledge: None,
                        release_date: None,
                        last_updated: None,
                        open_weights: Some(true),
                        input_modalities: if config.mmproj_file.is_some() {
                            Cow::Borrowed(&["text", "image"])
                        } else {
                            Cow::Borrowed(&["text"])
                        },
                        output_modalities: Cow::Borrowed(&["text"]),
                        cost: None,
                        limit: None,
                        status: None,
                        capabilities: config.capabilities.clone(),
                    }
                }
            })
            .collect();

        let mut capabilities = ProviderCapabilities::text_only();
        capabilities.developer_messages = true;

        ProviderInfo {
            name: "llamacpp".into(),
            default_model_id: Some(self.default_model.clone()),
            capabilities,
            models,
        }
    }
}

struct InferencePlan {
    mode: InferenceMode,
    max_tokens: usize,
    temperature: f32,
}

enum InferenceMode {
    Text {
        prompt: String,
        parser: Option<llama_cpp_2::model::ChatTemplateResult>,
    },
    Multimodal {
        prompt: String,
        bitmaps: Vec<MtmdBitmap>,
    },
}

async fn build_inference_plan(
    loaded: &LoadedModel,
    request: &Request,
) -> Result<InferencePlan, Error> {
    let max_tokens = request.options.max_output_tokens.unwrap_or(512) as usize;
    let temperature = request.options.temperature.unwrap_or(0.7) as f32;
    let image_count = request
        .messages
        .iter()
        .flat_map(|message| &message.content)
        .filter(|block| matches!(block, provider::ContentBlock::ImageUrl { .. }))
        .count();

    if image_count > 0 {
        if !request.tools.is_empty() {
            return Err(Error::Unsupported(
                "llamacpp multimodal requests do not currently support tool definitions".into(),
            ));
        }
        let mtmd = loaded.mtmd.as_ref().ok_or_else(|| {
            Error::Unsupported("llamacpp image input requires mmproj_file".into())
        })?;
        let (prompt, bitmaps) = build_multimodal_prompt(mtmd, request).await?;
        if !mtmd.support_vision() {
            return Err(Error::Unsupported(
                "configured llama.cpp multimodal context does not support vision".into(),
            ));
        }
        return Ok(InferencePlan {
            mode: InferenceMode::Multimodal { prompt, bitmaps },
            max_tokens,
            temperature,
        });
    }

    let template = loaded
        .model
        .chat_template(None)
        .map_err(|err| Error::Inference(format!("no chat template in model: {err}")))?;

    if request.tools.is_empty()
        && request.messages.iter().all(|message| {
            message.content.iter().all(|block| {
                matches!(
                    block,
                    provider::ContentBlock::Text { .. }
                        | provider::ContentBlock::Reasoning { .. }
                        | provider::ContentBlock::Refusal { .. }
                )
            })
        })
    {
        let chat = build_chat_messages(request)?;
        let prompt = loaded
            .model
            .apply_chat_template(&template, &chat, true)
            .map_err(|err| Error::Inference(format!("failed to apply chat template: {err}")))?;
        return Ok(InferencePlan {
            mode: InferenceMode::Text {
                prompt,
                parser: None,
            },
            max_tokens,
            temperature,
        });
    }

    let messages_json = serde_json::to_string(&map_oaicompat_messages(request)?)?;
    let tools_json = if request.tools.is_empty() {
        None
    } else {
        Some(serde_json::to_string(&map_oaicompat_tools(
            &request.tools,
        )?)?)
    };
    let tool_choice = match request.options.tool_choice.as_ref() {
        None | Some(provider::ToolChoice::Auto) => Some("auto".to_string()),
        Some(provider::ToolChoice::None) => Some("none".to_string()),
        Some(provider::ToolChoice::Required) => None,
        Some(provider::ToolChoice::Tool { name }) => Some(name.clone()),
    };

    let parser = loaded
        .model
        .apply_chat_template_oaicompat(
            &template,
            &OpenAIChatTemplateParams {
                messages_json: &messages_json,
                tools_json: tools_json.as_deref(),
                tool_choice: tool_choice.as_deref(),
                json_schema: None,
                grammar: None,
                reasoning_format: request
                    .options
                    .reasoning
                    .as_ref()
                    .and_then(|reasoning| reasoning.effort.as_deref()),
                chat_template_kwargs: None,
                add_generation_prompt: true,
                use_jinja: true,
                parallel_tool_calls: request.options.parallel_tool_calls.unwrap_or(true),
                enable_thinking: request.options.reasoning.is_some(),
                add_bos: true,
                add_eos: false,
                parse_tool_calls: !request.tools.is_empty(),
            },
        )
        .map_err(|err| {
            Error::Inference(format!(
                "failed to apply OpenAI-compatible chat template: {err}"
            ))
        })?;

    Ok(InferencePlan {
        mode: InferenceMode::Text {
            prompt: parser.prompt.clone(),
            parser: Some(parser),
        },
        max_tokens,
        temperature,
    })
}

fn run_inference(
    backend: &LlamaBackend,
    loaded: &LoadedModel,
    plan: InferencePlan,
    tx: &tokio::sync::mpsc::Sender<Result<Event, Error>>,
) -> Result<(), Error> {
    match plan.mode {
        InferenceMode::Text { prompt, parser } => run_text_inference(
            backend,
            &loaded.model,
            &prompt,
            parser.as_ref(),
            plan.max_tokens,
            plan.temperature,
            tx,
        ),
        InferenceMode::Multimodal { prompt, bitmaps } => {
            let mtmd = loaded.mtmd.as_ref().ok_or_else(|| {
                Error::Unsupported("llamacpp multimodal context not configured".into())
            })?;
            run_multimodal_inference(
                backend,
                &loaded.model,
                mtmd,
                &prompt,
                &bitmaps,
                plan.max_tokens,
                plan.temperature,
                tx,
            )
        }
    }
}

fn run_text_inference(
    backend: &LlamaBackend,
    model: &LlamaModel,
    prompt: &str,
    parser: Option<&llama_cpp_2::model::ChatTemplateResult>,
    max_tokens: usize,
    temperature: f32,
    tx: &tokio::sync::mpsc::Sender<Result<Event, Error>>,
) -> Result<(), Error> {
    let tokens = model
        .str_to_token(prompt, AddBos::Always)
        .map_err(|err| Error::Inference(format!("tokenization failed: {err}")))?;
    let prompt_len = tokens.len();
    let total_len = prompt_len + max_tokens;

    let ctx_params = LlamaContextParams::default()
        .with_n_ctx(NonZeroU32::new(total_len as u32))
        .with_n_batch(total_len as u32);
    let mut ctx = model
        .new_context(backend, ctx_params)
        .map_err(|err| Error::Inference(format!("failed to create context: {err}")))?;

    let mut batch = LlamaBatch::new(total_len, 1);
    let last_idx = tokens.len().saturating_sub(1) as i32;
    for (i, token) in (0_i32..).zip(tokens.iter().copied()) {
        batch
            .add(token, i, &[0], i == last_idx)
            .map_err(|err| Error::Inference(format!("batch add failed: {err}")))?;
    }
    ctx.decode(&mut batch)
        .map_err(|err| Error::Inference(format!("prompt decode failed: {err}")))?;

    generate_from_context(
        model,
        &mut ctx,
        &mut batch,
        prompt_len as u32,
        max_tokens,
        temperature,
        parser,
        tx,
    )
}

fn run_multimodal_inference(
    backend: &LlamaBackend,
    model: &LlamaModel,
    mtmd: &MtmdContext,
    prompt: &str,
    bitmaps: &[MtmdBitmap],
    max_tokens: usize,
    temperature: f32,
    tx: &tokio::sync::mpsc::Sender<Result<Event, Error>>,
) -> Result<(), Error> {
    let total_len = 4096 + max_tokens;
    let ctx_params = LlamaContextParams::default()
        .with_n_ctx(NonZeroU32::new(total_len as u32))
        .with_n_batch(total_len as u32);
    let mut ctx = model
        .new_context(backend, ctx_params)
        .map_err(|err| Error::Inference(format!("failed to create context: {err}")))?;

    let input = MtmdInputText {
        text: prompt.to_string(),
        add_special: true,
        parse_special: true,
    };
    let bitmap_refs = bitmaps.iter().collect::<Vec<_>>();
    let chunks = mtmd
        .tokenize(input, &bitmap_refs)
        .map_err(|err| Error::Inference(format!("failed to tokenize multimodal prompt: {err}")))?;
    let n_past = chunks
        .eval_chunks(mtmd, &ctx, 0, 0, total_len as i32, true)
        .map_err(|err| Error::Inference(format!("failed to evaluate multimodal prompt: {err}")))?;
    let prompt_len = n_past as u32;

    let mut batch = LlamaBatch::new(total_len, 1);
    generate_from_context(
        model,
        &mut ctx,
        &mut batch,
        prompt_len,
        max_tokens,
        temperature,
        None,
        tx,
    )
}

fn generate_from_context(
    model: &LlamaModel,
    ctx: &mut llama_cpp_2::context::LlamaContext<'_>,
    batch: &mut LlamaBatch,
    prompt_len: u32,
    max_tokens: usize,
    temperature: f32,
    parser: Option<&llama_cpp_2::model::ChatTemplateResult>,
    tx: &tokio::sync::mpsc::Sender<Result<Event, Error>>,
) -> Result<(), Error> {
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
    let mut n_cur = prompt_len as i32;
    let mut completion_tokens = 0u32;
    let mut emitted_text = false;
    let mut open_blocks = HashSet::new();
    let mut parser_state = parser.and_then(|result| result.streaming_state_oaicompat().ok());

    for _ in 0..max_tokens {
        let token = sampler.sample(ctx, batch.n_tokens() - 1);
        sampler.accept(token);
        if model.is_eog_token(token) {
            break;
        }

        completion_tokens += 1;
        let piece = model
            .token_to_piece(token, &mut decoder, true, None)
            .map_err(|err| Error::Inference(format!("detokenize failed: {err}")))?;

        if !piece.is_empty() {
            if let Some(state) = parser_state.as_mut() {
                let deltas = state.update(&piece, true).map_err(|err| {
                    Error::Inference(format!("failed to parse llama output: {err}"))
                })?;
                for delta in deltas {
                    for event in parse_oaicompat_delta(&delta, &mut open_blocks)? {
                        if tx.blocking_send(Ok(event)).is_err() {
                            return Ok(());
                        }
                    }
                }
            } else {
                if !emitted_text {
                    emitted_text = true;
                    open_blocks.insert("llamacpp-text:0".to_string());
                    let _ = tx.blocking_send(Ok(Event::BlockStart {
                        block: Block {
                            id: "llamacpp-text:0".into(),
                            output_index: 0,
                            kind: BlockKind::Text,
                            item_id: None,
                        },
                    }));
                }
                let _ = tx.blocking_send(Ok(Event::BlockDelta {
                    id: "llamacpp-text:0".into(),
                    delta: BlockDelta::Text { text: piece },
                }));
            }
        }

        batch.clear();
        batch
            .add(token, n_cur, &[0], true)
            .map_err(|err| Error::Inference(format!("batch add failed: {err}")))?;
        n_cur += 1;
        ctx.decode(batch)
            .map_err(|err| Error::Inference(format!("decode failed: {err}")))?;
    }

    if let Some(state) = parser_state.as_mut() {
        let deltas = state
            .update("", false)
            .map_err(|err| Error::Inference(format!("failed to finalize llama output: {err}")))?;
        for delta in deltas {
            for event in parse_oaicompat_delta(&delta, &mut open_blocks)? {
                if tx.blocking_send(Ok(event)).is_err() {
                    return Ok(());
                }
            }
        }
    }

    for id in open_blocks.drain() {
        let _ = tx.blocking_send(Ok(Event::BlockStop { id }));
    }
    let _ = tx.blocking_send(Ok(Event::Usage {
        usage: provider::Usage::with_totals(Some(prompt_len), Some(completion_tokens)),
    }));
    let _ = tx.blocking_send(Ok(Event::Completed {
        response_id: None,
        finish_reason: Some(FinishReason::Stop),
    }));

    Ok(())
}

fn build_chat_messages(request: &Request) -> Result<Vec<LlamaChatMessage>, Error> {
    request
        .messages
        .iter()
        .map(|message| {
            let role = match message.role {
                MessageRole::System | MessageRole::Developer => "system",
                MessageRole::User => "user",
                MessageRole::Assistant => "assistant",
            };
            if message.content.iter().any(|block| {
                matches!(
                    block,
                    provider::ContentBlock::ImageUrl { .. }
                        | provider::ContentBlock::ToolCall { .. }
                        | provider::ContentBlock::ToolResult { .. }
                )
            }) {
                return Err(Error::Unsupported(
                    "llamacpp plain text path only supports text-like content".into(),
                ));
            }
            LlamaChatMessage::new(role.to_string(), message.plain_text_lossy())
                .map_err(|err| Error::Inference(format!("invalid chat message: {err}")))
        })
        .collect()
}

async fn build_multimodal_prompt(
    mtmd: &MtmdContext,
    request: &Request,
) -> Result<(String, Vec<MtmdBitmap>), Error> {
    let mut prompt = String::new();
    let mut bitmaps = Vec::new();
    for message in &request.messages {
        let role = match message.role {
            MessageRole::System | MessageRole::Developer => "System",
            MessageRole::User => "User",
            MessageRole::Assistant => "Assistant",
        };
        prompt.push_str(role);
        prompt.push_str(": ");
        for block in &message.content {
            match block {
                provider::ContentBlock::Text { text }
                | provider::ContentBlock::Reasoning { text }
                | provider::ContentBlock::Refusal { text } => prompt.push_str(text),
                provider::ContentBlock::ImageUrl { url } => {
                    prompt.push_str("<__media__>");
                    bitmaps.push(load_bitmap(mtmd, url).await?);
                }
                provider::ContentBlock::ToolCall { .. }
                | provider::ContentBlock::ToolResult { .. } => {
                    return Err(Error::Unsupported(
                        "llamacpp multimodal path does not currently support tool history".into(),
                    ));
                }
            }
        }
        prompt.push('\n');
    }
    prompt.push_str("Assistant: ");
    Ok((prompt, bitmaps))
}

fn map_oaicompat_messages(request: &Request) -> Result<Vec<serde_json::Value>, Error> {
    let mut messages = Vec::new();
    for message in &request.messages {
        let role = match message.role {
            MessageRole::System | MessageRole::Developer => "system",
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
        };

        let text_parts = message
            .content
            .iter()
            .filter_map(|block| match block {
                provider::ContentBlock::Text { text }
                | provider::ContentBlock::Reasoning { text }
                | provider::ContentBlock::Refusal { text } => {
                    Some(serde_json::json!({"type":"text","text": text}))
                }
                provider::ContentBlock::ImageUrl { url } => {
                    Some(serde_json::json!({"type":"image_url","image_url":{"url": url}}))
                }
                provider::ContentBlock::ToolCall { .. }
                | provider::ContentBlock::ToolResult { .. } => None,
            })
            .collect::<Vec<_>>();

        let mut tool_calls = Vec::new();
        for block in &message.content {
            if let provider::ContentBlock::ToolCall { id, name, input } = block {
                tool_calls.push(serde_json::json!({
                    "id": id,
                    "type": "function",
                    "function": {
                        "name": name,
                        "arguments": serde_json::to_string(input)?,
                    }
                }));
            }
        }

        let tool_results = message
            .content
            .iter()
            .filter_map(|block| match block {
                provider::ContentBlock::ToolResult {
                    call_id, output, ..
                } => Some(serde_json::json!({
                    "role": "tool",
                    "tool_call_id": call_id,
                    "content": serde_json::to_string(output).unwrap_or_default(),
                })),
                _ => None,
            })
            .collect::<Vec<_>>();

        if !tool_results.is_empty() {
            messages.extend(tool_results);
            continue;
        }

        if !tool_calls.is_empty() {
            messages.push(serde_json::json!({
                "role": role,
                "content": text_parts,
                "tool_calls": tool_calls,
            }));
        } else if text_parts.len() == 1 {
            messages.push(serde_json::json!({
                "role": role,
                "content": text_parts,
            }));
        } else {
            messages.push(serde_json::json!({
                "role": role,
                "content": text_parts,
            }));
        }
    }
    Ok(messages)
}

fn map_oaicompat_tools(
    tools: &[provider::ToolDefinition],
) -> Result<Vec<serde_json::Value>, Error> {
    Ok(tools
        .iter()
        .map(|tool| {
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": tool.name,
                    "description": tool.description.clone().unwrap_or_default(),
                    "parameters": tool.input_schema,
                }
            })
        })
        .collect())
}

fn parse_oaicompat_delta(
    delta: &str,
    open_blocks: &mut HashSet<String>,
) -> Result<Vec<Event>, Error> {
    let mut events = Vec::new();
    let value: serde_json::Value = serde_json::from_str(delta)?;
    let choice = value
        .get("choices")
        .and_then(|choices| choices.as_array())
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.get("delta"))
        .cloned()
        .unwrap_or(serde_json::Value::Null);

    if let Some(content) = choice.get("content").and_then(|content| content.as_str()) {
        let block_id = "llamacpp-text:0".to_string();
        if open_blocks.insert(block_id.clone()) {
            events.push(Event::BlockStart {
                block: Block {
                    id: block_id.clone(),
                    output_index: 0,
                    kind: BlockKind::Text,
                    item_id: None,
                },
            });
        }
        if !content.is_empty() {
            events.push(Event::BlockDelta {
                id: block_id,
                delta: BlockDelta::Text {
                    text: content.to_string(),
                },
            });
        }
    }

    if let Some(reasoning) = choice
        .get("reasoning_content")
        .and_then(|reasoning| reasoning.as_str())
    {
        let block_id = "llamacpp-reasoning:0".to_string();
        if open_blocks.insert(block_id.clone()) {
            events.push(Event::BlockStart {
                block: Block {
                    id: block_id.clone(),
                    output_index: 0,
                    kind: BlockKind::Reasoning,
                    item_id: None,
                },
            });
        }
        if !reasoning.is_empty() {
            events.push(Event::BlockDelta {
                id: block_id,
                delta: BlockDelta::Reasoning {
                    text: reasoning.to_string(),
                },
            });
        }
    }

    if let Some(tool_calls) = choice
        .get("tool_calls")
        .and_then(|tool_calls| tool_calls.as_array())
    {
        for (index, tool_call) in tool_calls.iter().enumerate() {
            let call_id = tool_call
                .get("id")
                .and_then(|id| id.as_str())
                .unwrap_or("call");
            let name = tool_call
                .get("function")
                .and_then(|function| function.get("name"))
                .and_then(|name| name.as_str())
                .map(|name| name.to_string());
            let arguments = tool_call
                .get("function")
                .and_then(|function| function.get("arguments"))
                .and_then(|arguments| arguments.as_str())
                .unwrap_or("")
                .to_string();
            let block_id = format!("llamacpp-tool:{index}:{call_id}");
            if open_blocks.insert(block_id.clone()) {
                events.push(Event::BlockStart {
                    block: Block {
                        id: block_id.clone(),
                        output_index: index as u32,
                        kind: BlockKind::ToolCall {
                            name,
                            call_id: Some(call_id.to_string()),
                        },
                        item_id: None,
                    },
                });
            }
            if !arguments.is_empty() {
                events.push(Event::BlockDelta {
                    id: block_id,
                    delta: BlockDelta::Json {
                        partial_json: arguments,
                    },
                });
            }
        }
    }

    Ok(events)
}

fn resolve_file(model_id: &str, file: &str) -> Result<PathBuf, Error> {
    let path = PathBuf::from(model_id);
    if path.is_dir() {
        return Ok(path.join(file));
    }

    hf_hub::api::sync::ApiBuilder::new()
        .with_progress(true)
        .build()
        .map_err(|err| Error::Inference(format!("failed to init HF API: {err}")))?
        .model(model_id.to_string())
        .get(file)
        .map_err(|err| Error::Inference(format!("failed to download {model_id}/{file}: {err}")))
}

async fn load_bitmap(mtmd: &MtmdContext, url: &str) -> Result<MtmdBitmap, Error> {
    if let Some(data) = url.strip_prefix("data:") {
        let (_, encoded) = data
            .split_once(',')
            .ok_or_else(|| Error::Unsupported("invalid image data URL".into()))?;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .map_err(|err| Error::Unsupported(format!("invalid image data URL: {err}")))?;
        MtmdBitmap::from_buffer(mtmd, &bytes)
            .map_err(|err| Error::Unsupported(format!("failed to decode image: {err}")))
    } else {
        let bytes = reqwest::get(url)
            .await
            .map_err(|err| Error::Inference(format!("failed to fetch image: {err}")))?
            .bytes()
            .await
            .map_err(|err| Error::Inference(format!("failed to read image response: {err}")))?;
        MtmdBitmap::from_buffer(mtmd, &bytes)
            .map_err(|err| Error::Unsupported(format!("failed to decode image: {err}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LlamaCppModelPreset;

    #[test]
    fn info_reports_default_model() {
        let provider = LlamaCppProvider::new(
            vec![(
                "qwen3.5-0.8b".into(),
                LlamaCppConfig::new("repo", "model.gguf"),
            )],
            "qwen3.5-0.8b",
        )
        .unwrap();

        let info = provider.info();
        assert_eq!(info.default_model_id.as_deref(), Some("qwen3.5-0.8b"));
        assert_eq!(info.models.len(), 1);
    }

    #[test]
    fn info_prefers_preset_model_metadata() {
        let preset = LlamaCppModelPreset::QWEN35_0_8B;
        let provider = LlamaCppProvider::new(
            vec![(preset.id().to_owned(), preset.into_config())],
            "qwen3.5-0.8b",
        )
        .unwrap();

        let info = provider.info();
        let model = info
            .models
            .first()
            .expect("provider should expose model info");
        let capabilities = model
            .capabilities
            .as_ref()
            .expect("preset should provide model capabilities");

        assert_eq!(model.id.as_ref(), "qwen3.5-0.8b");
        assert_eq!(model.name.as_ref(), "Qwen3.5 0.8B");
        assert_eq!(model.family.as_deref(), Some("qwen3.5"));
        assert_eq!(model.input_modalities.as_ref(), ["text", "image"]);
        assert!(!model.tool_call);
        assert!(model.attachment);
        assert!(capabilities.input_image_urls);
        assert!(capabilities.reasoning_blocks);
    }

    #[test]
    fn parses_openai_compatible_text_delta() {
        let events = parse_oaicompat_delta(
            r#"{"choices":[{"delta":{"content":"hello"}}]}"#,
            &mut HashSet::new(),
        )
        .unwrap();
        assert!(
            events
                .iter()
                .any(|event| matches!(event, Event::BlockDelta { .. }))
        );
    }
}
