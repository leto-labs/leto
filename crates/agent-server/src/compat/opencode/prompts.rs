pub(super) enum PromptKind {
    Message,
}

pub(super) async fn prompt_like(
    server: &AppState,
    session_id: &str,
    body: PromptRequest,
    kind: PromptKind,
) -> Response {
    let internal = match parse_compat_session_id(session_id) {
        Ok(id) => id,
        Err(response) => return response.into_response(),
    };
    let input = match kind {
        PromptKind::Message => prompt_input_from_body(&body),
    };
    let stream = match server.core().turn(internal, input).await {
        Ok(stream) => stream,
        Err(error) => return compat_error("core_error", error.to_string()),
    };
    futures::pin_mut!(stream);
    while stream.next().await.is_some() {}
    match server.core().messages(internal).await {
        Ok(messages) => match messages.len().checked_sub(1) {
            Some(index) => {
                let parent_id = index.checked_sub(1).map(|parent| messages[parent].id);
                match compat_assistant_message_with_parts(&messages[index], parent_id) {
                    Some(message) => Json(message).into_response(),
                    None => Json(AssistantMessageWithPartsDoc {
                        info: AssistantMessageDoc {
                            id: compat_message_id(messages[index].id),
                            session_id: compat_session_id(messages[index].session_id),
                            role: AssistantMessageRoleDoc::Assistant,
                            time: MessageTimeCreatedCompletedDoc {
                                created: messages[index].created_at.timestamp_millis() as f64,
                                completed: Some(
                                    messages[index].created_at.timestamp_millis() as f64
                                ),
                            },
                            error: None,
                            parent_id: compat_message_id(parent_id.unwrap_or(messages[index].id)),
                            model_id: "compat".to_owned(),
                            provider_id: "compat".to_owned(),
                            mode: "chat".to_owned(),
                            agent: "general".to_owned(),
                            path: MessagePathDoc {
                                cwd: default_directory_string(),
                                root: default_directory_string(),
                            },
                            summary: None,
                            cost: 0.0,
                            tokens: MessageTokensDoc {
                                total: Some(0.0),
                                input: 0.0,
                                output: 0.0,
                                reasoning: 0.0,
                                cache: MessageTokensCacheDoc {
                                    read: 0.0,
                                    write: 0.0,
                                },
                            },
                            structured: None,
                            variant: None,
                            finish: None,
                        },
                        parts: compat_parts(
                            messages[index].id,
                            messages[index].session_id,
                            &messages[index].message,
                        ),
                    })
                    .into_response(),
                }
            }
            None => Json(json!({})).into_response(),
        },
        Err(error) => compat_error("core_error", error.to_string()),
    }
}

pub(super) async fn prompt_command_like(
    server: &AppState,
    session_id: &str,
    body: CommandRequest,
) -> Response {
    let prompt = format!("/{} {}", body.command, body.arguments)
        .trim()
        .to_owned();
    prompt_text_like(server, session_id, prompt).await
}

pub(super) async fn prompt_shell_like(
    server: &AppState,
    session_id: &str,
    body: ShellRequest,
) -> Response {
    let internal = match parse_compat_session_id(session_id) {
        Ok(id) => id,
        Err(response) => return response.into_response(),
    };
    let input = vec![Message::user_text(format!(
        "Run shell command: {}",
        body.command
    ))];
    let stream = match server.core().turn(internal, input).await {
        Ok(stream) => stream,
        Err(error) => return compat_error("core_error", error.to_string()),
    };
    futures::pin_mut!(stream);
    while stream.next().await.is_some() {}
    match server.core().messages(internal).await {
        Ok(messages) => match messages.len().checked_sub(1) {
            Some(index) => {
                let parent_id = index.checked_sub(1).map(|parent| messages[parent].id);
                match compat_message(&messages[index], parent_id) {
                    MessageDoc::Assistant(message) => Json(message).into_response(),
                    MessageDoc::User(_) => compat_error(
                        "invalid_state",
                        "latest message is not an assistant message",
                    ),
                }
            }
            None => Json(json!({})).into_response(),
        },
        Err(error) => compat_error("core_error", error.to_string()),
    }
}

pub(super) async fn prompt_text_like(
    server: &AppState,
    session_id: &str,
    prompt: String,
) -> Response {
    let internal = match parse_compat_session_id(session_id) {
        Ok(id) => id,
        Err(response) => return response.into_response(),
    };
    let input = vec![Message::user_text(prompt)];
    let stream = match server.core().turn(internal, input).await {
        Ok(stream) => stream,
        Err(error) => return compat_error("core_error", error.to_string()),
    };
    futures::pin_mut!(stream);
    while stream.next().await.is_some() {}
    match server.core().messages(internal).await {
        Ok(messages) => match messages.len().checked_sub(1) {
            Some(index) => {
                let parent_id = index.checked_sub(1).map(|parent| messages[parent].id);
                match compat_assistant_message_with_parts(&messages[index], parent_id) {
                    Some(message) => Json(message).into_response(),
                    None => compat_error(
                        "invalid_state",
                        "latest message is not an assistant message",
                    ),
                }
            }
            None => Json(json!({})).into_response(),
        },
        Err(error) => compat_error("core_error", error.to_string()),
    }
}

pub(super) fn prompt_input_from_body(body: &PromptRequest) -> Vec<Message> {
    let blocks = body
        .parts
        .iter()
        .map(|part| match part {
            PromptPartInputDoc::Text(part) => ContentBlock::text(part.text.clone()),
            PromptPartInputDoc::File(part) => ContentBlock::ImageUrl {
                url: part.url.clone(),
            },
            PromptPartInputDoc::Agent(part) => ContentBlock::text(format!("@{}", part.name)),
            PromptPartInputDoc::Subtask(part) => {
                ContentBlock::text(format!("{}\n{}", part.description, part.prompt))
            }
        })
        .collect::<Vec<_>>();
    let blocks = if blocks.is_empty() {
        vec![ContentBlock::text("")]
    } else {
        blocks
    };
    vec![Message::new(MessageRole::User, blocks)]
}

pub(super) async fn mutate_message_part(
    server: &AppState,
    session_id: &str,
    message_id: &str,
    part_id: &str,
    body: Option<Value>,
) -> Response {
    let session_id = match parse_compat_session_id(session_id) {
        Ok(id) => id,
        Err(response) => return response.into_response(),
    };
    let message_id = match parse_compat_message_id(message_id) {
        Ok(id) => id,
        Err(response) => return response.into_response(),
    };
    let part_index = match parse_compat_part_index(message_id, part_id) {
        Ok(index) => index,
        Err(response) => return response.into_response(),
    };
    let mut messages = match server.core().messages(session_id).await {
        Ok(messages) => messages,
        Err(error) => return compat_error("core_error", error.to_string()),
    };
    let Some(message) = messages.iter_mut().find(|message| message.id == message_id) else {
        return compat_error("not_found", "message not found");
    };
    if part_index >= message.message.content.len() {
        return compat_error("not_found", "part not found");
    }
    match body {
        Some(ref body) => {
            message.message.content[part_index] = content_block_from_part(body);
        }
        None => {
            message.message.content.remove(part_index);
        }
    }
    match server
        .core()
        .store()
        .messages()
        .replace_for_session(session_id, messages)
        .await
    {
        Ok(updated) => {
            if body.is_some() {
                match updated
                    .into_iter()
                    .find(|stored| stored.id == message_id)
                    .map(|stored| compat_parts(stored.id, stored.session_id, &stored.message))
                    .and_then(|parts| parts.into_iter().nth(part_index))
                {
                    Some(part) => Json(part).into_response(),
                    None => compat_error("not_found", "part not found"),
                }
            } else {
                Json(true).into_response()
            }
        }
        Err(error) => compat_error("store_error", error.to_string()),
    }
}

fn content_block_from_part(body: &Value) -> ContentBlock {
    match body.get("type").and_then(Value::as_str) {
        Some("file") => ContentBlock::ImageUrl {
            url: body
                .get("url")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
        },
        _ => ContentBlock::text(body.get("text").and_then(Value::as_str).unwrap_or_default()),
    }
}
