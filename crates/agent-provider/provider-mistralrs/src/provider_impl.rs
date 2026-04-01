use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use async_stream::stream;
use base64::Engine;
use futures::future::BoxFuture;
use image::DynamicImage;
use mistralrs::{
    CalledFunction, Function, GgufModelBuilder, Model, RequestBuilder, Response, StopTokens,
    TextMessageRole, Tool, ToolCallResponse, ToolCallType, ToolChoice as MistralToolChoice,
    ToolType,
};
use provider::{
    Block, BlockDelta, BlockKind, Error, Event, EventStream, FinishReason, Message, MessageRole,
    ModelInfo, Provider, ProviderCapabilities, ProviderInfo, Request, Usage,
};
use tokio::sync::RwLock;

use crate::config::{DevicePreference, MistralRsConfig};

/// Shared `provider::Provider` adapter backed by local mistral.rs inference.
pub struct MistralRsProvider {
    configs: HashMap<String, MistralRsConfig>,
    models: RwLock<HashMap<String, Arc<Model>>>,
    default_model: String,
}

impl MistralRsProvider {
    /// Creates a provider from named configs and a default model identifier.
    pub fn new(configs: Vec<(String, MistralRsConfig)>, default_model: impl Into<String>) -> Self {
        Self {
            configs: configs.into_iter().collect(),
            models: RwLock::new(HashMap::new()),
            default_model: default_model.into(),
        }
    }

    /// Adds another named model config to the provider registry.
    pub fn add(mut self, name: impl Into<String>, config: MistralRsConfig) -> Self {
        self.configs.insert(name.into(), config);
        self
    }

    /// Resolves and caches a named model without issuing an inference request.
    pub async fn preload(&self, name: &str) -> Result<(), Error> {
        self.resolve_model(name).await.map(|_| ())
    }

    async fn resolve_model(&self, name: &str) -> Result<Arc<Model>, Error> {
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
        let model = Arc::new(load_model(config).await?);

        let mut cache = self.models.write().await;
        Ok(cache
            .entry(name.to_owned())
            .or_insert_with(|| model.clone())
            .clone())
    }
}

async fn load_model(config: &MistralRsConfig) -> Result<Model, Error> {
    let mut builder = GgufModelBuilder::new(config.model_id.clone(), config.gguf_files.clone());
    if matches!(config.device, DevicePreference::Cpu) {
        builder = builder.with_force_cpu();
    }

    builder
        .build()
        .await
        .map_err(|err| Error::Inference(format!("failed to load model: {err}")))
}

impl Provider for MistralRsProvider {
    fn stream<'a>(&'a self, request: &'a Request) -> BoxFuture<'a, Result<EventStream<'a>, Error>> {
        Box::pin(async move {
            let model_name = request
                .model
                .clone()
                .unwrap_or_else(|| self.default_model.clone());
            let model = self.resolve_model(&model_name).await?;
            let mistral_request = build_request(model.as_ref(), request).await?;
            let output = stream! {
                let model = model;
                let mut stream = match model.stream_chat_request(mistral_request).await {
                    Ok(stream) => stream,
                    Err(err) => {
                        yield Err(Error::Inference(format!("stream request failed: {err}")));
                        return;
                    }
                };
                let mut opened = HashSet::<String>::new();
                let mut completed = false;
                let mut failed = false;

                yield Ok(Event::ResponseStart {
                    response_id: None,
                    model: Some(model_name.clone()),
                });

                while let Some(chunk) = stream.next().await {
                    match chunk {
                        Response::Chunk(chunk) => {
                            if let Some(usage) = chunk.usage.as_ref().map(map_usage) {
                                yield Ok(Event::Usage { usage });
                            }
                            if let Some(choice) = chunk.choices.first() {
                                for event in emit_chunk_delta(choice.delta.content.as_ref(), choice.delta.reasoning_content.as_ref(), choice.delta.tool_calls.as_deref(), &mut opened) {
                                    yield Ok(event);
                                }
                            }
                        }
                        Response::Done(done) => {
                            if let Some(choice) = done.choices.first() {
                                for event in emit_chunk_delta(choice.message.content.as_ref(), choice.message.reasoning_content.as_ref(), choice.message.tool_calls.as_deref(), &mut opened) {
                                    yield Ok(event);
                                }
                                yield Ok(Event::Usage { usage: map_usage(&done.usage) });
                                for id in opened.drain() {
                                    yield Ok(Event::BlockStop { id });
                                }
                                yield Ok(Event::Completed {
                                    response_id: Some(done.id.clone()),
                                    finish_reason: Some(map_finish_reason(&choice.finish_reason)),
                                });
                                completed = true;
                            }
                            break;
                        }
                        Response::ModelError(message, _) => {
                            failed = true;
                            yield Err(Error::Inference(message));
                            break;
                        }
                        Response::InternalError(err) | Response::ValidationError(err) => {
                            failed = true;
                            yield Err(Error::Inference(err.to_string()));
                            break;
                        }
                        _ => {}
                    }
                }

                if !completed && !failed {
                    for id in opened.drain() {
                        yield Ok(Event::BlockStop { id });
                    }
                    yield Ok(Event::Completed {
                        response_id: None,
                        finish_reason: None,
                    });
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
                        attachment: config
                            .capabilities
                            .as_ref()
                            .map(|capabilities| capabilities.input_image_urls)
                            .unwrap_or(false),
                        structured_output: None,
                        temperature: Some(true),
                        knowledge: None,
                        release_date: None,
                        last_updated: None,
                        open_weights: Some(true),
                        input_modalities: if config
                            .capabilities
                            .as_ref()
                            .map(|capabilities| capabilities.input_image_urls)
                            .unwrap_or(false)
                        {
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
            name: "mistralrs".into(),
            default_model_id: Some(self.default_model.clone()),
            capabilities,
            models,
        }
    }
}

async fn build_request(model: &Model, request: &Request) -> Result<RequestBuilder, Error> {
    let mut builder = RequestBuilder::new();

    for message in &request.messages {
        builder = add_message(builder, model, message).await?;
    }

    if !request.tools.is_empty() {
        let tools = request
            .tools
            .iter()
            .map(map_tool_definition)
            .collect::<Result<Vec<_>, _>>()?;
        let tool_choice = map_tool_choice(request.options.tool_choice.as_ref(), &request.tools)?;
        builder = builder.set_tools(tools).set_tool_choice(tool_choice);
    }

    if let Some(temperature) = request.options.temperature {
        builder = builder.set_sampler_temperature(temperature);
    }
    if let Some(top_p) = request.options.top_p {
        builder = builder.set_sampler_topp(top_p);
    }
    if let Some(top_k) = request.options.top_k {
        builder = builder.set_sampler_topk(top_k as usize);
    }
    if let Some(max_tokens) = request.options.max_output_tokens {
        builder = builder.set_sampler_max_len(max_tokens as usize);
    }
    if !request.options.stop_sequences.is_empty() {
        builder =
            builder.set_sampler_stop_toks(StopTokens::Seqs(request.options.stop_sequences.clone()));
    }
    if request.options.reasoning.is_some() {
        builder = builder.enable_thinking(true);
    }
    if matches!(request.options.parallel_tool_calls, Some(false)) {
        return Err(Error::Unsupported(
            "mistralrs does not expose a parallel tool-call toggle through this adapter".into(),
        ));
    }

    Ok(builder)
}

async fn add_message(
    builder: RequestBuilder,
    model: &Model,
    message: &Message,
) -> Result<RequestBuilder, Error> {
    let role = map_role(message.role);
    let mut text_parts = Vec::new();
    let mut images = Vec::new();
    let mut tool_calls = Vec::new();
    let mut tool_results = Vec::new();

    for block in &message.content {
        match block {
            provider::ContentBlock::Text { text }
            | provider::ContentBlock::Reasoning { text }
            | provider::ContentBlock::Refusal { text } => text_parts.push(text.clone()),
            provider::ContentBlock::ImageUrl { url } => {
                images.push(load_image(url).await?);
            }
            provider::ContentBlock::ToolCall {
                id,
                call_id,
                name,
                input,
            } => {
                tool_calls.push(ToolCallResponse {
                    index: tool_calls.len(),
                    id: call_id.clone().unwrap_or_else(|| id.clone()),
                    tp: ToolCallType::Function,
                    function: CalledFunction {
                        name: name.clone(),
                        arguments: serde_json::to_string(input)?,
                    },
                });
            }
            provider::ContentBlock::ToolResult {
                call_id, output, ..
            } => {
                tool_results.push((call_id.clone(), serde_json::to_string(output)?));
            }
        }
    }

    if !tool_results.is_empty() {
        if tool_results.len() != message.content.len() {
            return Err(Error::Unsupported(
                "mistralrs tool result messages must contain only tool_result blocks".into(),
            ));
        }

        let mut builder = builder;
        for (call_id, output) in tool_results {
            builder = builder.add_tool_message(output, call_id);
        }
        return Ok(builder);
    }

    if !tool_calls.is_empty() {
        if message.role != MessageRole::Assistant {
            return Err(Error::Unsupported(
                "mistralrs tool_call blocks must appear on assistant messages".into(),
            ));
        }
        return Ok(builder.add_message_with_tool_call(role, text_parts.join("\n"), tool_calls));
    }

    if !images.is_empty() {
        builder
            .add_image_message(role, text_parts.join("\n"), images, model)
            .map_err(|err| Error::Unsupported(err.to_string()))
    } else {
        Ok(builder.add_message(role, text_parts.join("\n")))
    }
}

fn map_role(role: MessageRole) -> TextMessageRole {
    match role {
        MessageRole::System | MessageRole::Developer => TextMessageRole::System,
        MessageRole::User => TextMessageRole::User,
        MessageRole::Assistant => TextMessageRole::Assistant,
    }
}

fn map_tool_definition(tool: &provider::ToolDefinition) -> Result<Tool, Error> {
    let parameters = match &tool.input_schema {
        serde_json::Value::Object(map) => map
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect(),
        _ => {
            return Err(Error::Unsupported(
                "mistralrs tool schemas must serialize to a JSON object".into(),
            ));
        }
    };

    Ok(Tool {
        tp: ToolType::Function,
        function: Function {
            description: tool.description.clone(),
            name: tool.name.clone(),
            parameters: Some(parameters),
        },
    })
}

fn map_tool_choice(
    choice: Option<&provider::ToolChoice>,
    tools: &[provider::ToolDefinition],
) -> Result<MistralToolChoice, Error> {
    Ok(match choice {
        None | Some(provider::ToolChoice::Auto) => MistralToolChoice::Auto,
        Some(provider::ToolChoice::None) => MistralToolChoice::None,
        Some(provider::ToolChoice::Required) => {
            return Err(Error::Unsupported(
                "mistralrs does not expose a required-tool mode through this adapter".into(),
            ));
        }
        Some(provider::ToolChoice::Tool { name }) => {
            let tool = tools
                .iter()
                .find(|tool| tool.name == *name)
                .ok_or_else(|| Error::Configuration(format!("unknown tool requested: {name}")))?;
            MistralToolChoice::Tool(map_tool_definition(tool)?)
        }
    })
}

async fn load_image(url: &str) -> Result<DynamicImage, Error> {
    let bytes = if let Some(data) = url.strip_prefix("data:") {
        let (_, encoded) = data
            .split_once(',')
            .ok_or_else(|| Error::Unsupported("invalid data URL".into()))?;
        base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .map_err(|err| Error::Unsupported(format!("invalid image data URL: {err}")))?
    } else {
        reqwest::get(url)
            .await
            .map_err(|err| Error::Inference(format!("failed to fetch image: {err}")))?
            .bytes()
            .await
            .map_err(|err| Error::Inference(format!("failed to read image response: {err}")))?
            .to_vec()
    };

    image::load_from_memory(&bytes)
        .map_err(|err| Error::Unsupported(format!("failed to decode image: {err}")))
}

fn map_usage(usage: &mistralrs::Usage) -> Usage {
    Usage::with_totals(
        Some(usage.prompt_tokens as u32),
        Some(usage.completion_tokens as u32),
    )
}

fn map_finish_reason(reason: &str) -> FinishReason {
    match reason {
        "stop" => FinishReason::Stop,
        "length" => FinishReason::MaxTokens,
        "tool_calls" | "tool_call" => FinishReason::ToolCall,
        other => FinishReason::Unknown(other.to_string()),
    }
}

fn tool_call_block_id(tool_call: &ToolCallResponse) -> String {
    format!("mistralrs-tool:{}:{}", tool_call.index, tool_call.id)
}

fn emit_chunk_delta(
    content: Option<&String>,
    reasoning: Option<&String>,
    tool_calls: Option<&[ToolCallResponse]>,
    opened: &mut HashSet<String>,
) -> Vec<Event> {
    let mut events = Vec::new();

    if let Some(reasoning) = reasoning {
        if !reasoning.is_empty() {
            let block_id = "mistralrs-reasoning:0".to_string();
            if opened.insert(block_id.clone()) {
                events.push(Event::BlockStart {
                    block: Block {
                        id: block_id.clone(),
                        output_index: 0,
                        kind: BlockKind::Reasoning,
                        item_id: None,
                    },
                });
            }
            events.push(Event::BlockDelta {
                id: block_id,
                delta: BlockDelta::Reasoning {
                    text: reasoning.clone(),
                },
            });
        }
    }

    if let Some(content) = content {
        if !content.is_empty() {
            let block_id = "mistralrs-text:0".to_string();
            if opened.insert(block_id.clone()) {
                events.push(Event::BlockStart {
                    block: Block {
                        id: block_id.clone(),
                        output_index: 0,
                        kind: BlockKind::Text,
                        item_id: None,
                    },
                });
            }
            events.push(Event::BlockDelta {
                id: block_id,
                delta: BlockDelta::Text {
                    text: content.clone(),
                },
            });
        }
    }

    if let Some(tool_calls) = tool_calls {
        for tool_call in tool_calls {
            let block_id = tool_call_block_id(tool_call);
            if opened.insert(block_id.clone()) {
                events.push(Event::BlockStart {
                    block: Block {
                        id: block_id.clone(),
                        output_index: tool_call.index as u32,
                        kind: BlockKind::ToolCall {
                            name: Some(tool_call.function.name.clone()),
                            call_id: Some(tool_call.id.clone()),
                        },
                        item_id: None,
                    },
                });
            }
            if !tool_call.function.arguments.is_empty() {
                events.push(Event::BlockDelta {
                    id: block_id,
                    delta: BlockDelta::Json {
                        partial_json: tool_call.function.arguments.clone(),
                    },
                });
            }
        }
    }

    events
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MistralRsModelPreset;
    use provider::{ToolChoice, ToolDefinition};

    #[test]
    fn info_uses_default_model_and_capability_baseline() {
        let provider = MistralRsProvider::new(
            vec![(
                "qwen3-0.6b".into(),
                MistralRsConfig::new("repo", vec!["model.gguf".into()]),
            )],
            "qwen3-0.6b",
        );

        let info = provider.info();
        assert_eq!(info.default_model_id.as_deref(), Some("qwen3-0.6b"));
        assert!(info.capabilities.input_text);
        assert!(info.capabilities.developer_messages);
        assert_eq!(info.models.len(), 1);
    }

    #[test]
    fn info_prefers_preset_model_metadata() {
        let preset = MistralRsModelPreset::QWEN3_0_6B;
        let provider = MistralRsProvider::new(
            vec![(preset.id().to_owned(), preset.into_config())],
            "qwen3-0.6b",
        );

        let info = provider.info();
        let model = info
            .models
            .first()
            .expect("provider should expose model info");
        let capabilities = model
            .capabilities
            .as_ref()
            .expect("preset should provide model capabilities");

        assert_eq!(model.id.as_ref(), "qwen3-0.6b");
        assert_eq!(model.name.as_ref(), "Qwen3 0.6B");
        assert_eq!(model.family.as_deref(), Some("qwen3"));
        assert_eq!(model.input_modalities.as_ref(), ["text"]);
        assert!(!model.tool_call);
        assert!(capabilities.reasoning_blocks);
    }

    #[test]
    fn maps_specific_tool_choice() {
        let tools = vec![ToolDefinition::new(
            "echo",
            "Echo text",
            serde_json::json!({"type":"object"}),
        )];
        let choice = map_tool_choice(Some(&ToolChoice::tool("echo")), &tools).unwrap();
        assert!(matches!(choice, MistralToolChoice::Tool(_)));
    }

    #[test]
    fn maps_finish_reason_strings() {
        assert!(matches!(map_finish_reason("stop"), FinishReason::Stop));
        assert!(matches!(
            map_finish_reason("length"),
            FinishReason::MaxTokens
        ));
        assert!(matches!(
            map_finish_reason("tool_calls"),
            FinishReason::ToolCall
        ));
    }
}
