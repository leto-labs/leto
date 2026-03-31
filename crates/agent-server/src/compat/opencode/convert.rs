pub(super) async fn filtered_sessions(
    server: &AppState,
    params: &ExperimentalSessionListQueryDoc,
) -> Result<Vec<(Session, Option<Project>, CompatSessionMeta)>, Response> {
    let sessions = server
        .core()
        .store()
        .sessions()
        .list()
        .await
        .map_err(|error| compat_error("store_error", error.to_string()))?;
    let directory_filter = params.directory.clone();
    let roots_only = params.roots.unwrap_or(false);
    let search = params.search.as_ref().map(|value| value.to_lowercase());
    let start = params.start.map(|value| value as i64);
    let archived = params.archived.unwrap_or(false);
    let limit = params.limit.map(|value| value as usize);
    let mut rows = Vec::new();
    for session in sessions {
        let project = server.core().project(session.project_id).await.ok();
        let meta = session_meta(server, session.id).await;
        if roots_only && meta.parent_id.is_some() {
            continue;
        }
        if !archived && meta.archived_at.is_some() {
            continue;
        }
        if let Some(start) = start
            && session.updated_at.timestamp_millis() < start
        {
            continue;
        }
        if let Some(search) = &search {
            let title = session.title.clone().unwrap_or_default().to_lowercase();
            if !title.contains(search) {
                continue;
            }
        }
        if let Some(directory) = &directory_filter {
            let session_directory = project
                .as_ref()
                .and_then(|project| project.root.as_ref())
                .map(|root| root.display().to_string())
                .unwrap_or_else(default_directory_string);
            if &session_directory != directory {
                continue;
            }
        }
        rows.push((session, project, meta));
    }
    rows.sort_by_key(|(session, _, _)| std::cmp::Reverse(session.updated_at));
    if let Some(limit) = limit {
        rows.truncate(limit);
    }
    Ok(rows)
}

pub(super) async fn resolve_current_project(
    server: &AppState,
    params: &CompatQuery,
) -> Result<Project, Response> {
    let root = query_directory(params.directory.as_deref());
    server
        .core()
        .resolve_or_create_project(root)
        .await
        .map_err(|error| compat_error("core_error", error.to_string()))
}

pub(super) async fn session_meta(server: &AppState, session_id: SessionId) -> CompatSessionMeta {
    server
        .compat()
        .session_meta
        .read()
        .await
        .get(&compat_session_id(session_id))
        .cloned()
        .unwrap_or_else(|| CompatSessionMeta {
            permission: json!([]),
            ..CompatSessionMeta::default()
        })
}

pub(super) async fn compat_project(
    project: &Project,
    compat: &Arc<crate::compat::opencode::state::CompatState>,
) -> ProjectDoc {
    let sandboxes = compat
        .worktrees
        .read()
        .await
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    let fallback = default_directory();
    let root = project.root.as_deref().unwrap_or(fallback.as_path());
    ProjectDoc {
        id: project.id.to_string(),
        worktree: project
            .root
            .as_ref()
            .map(|root| root.display().to_string())
            .unwrap_or_else(default_directory_string),
        vcs: is_git_root(root).then_some(ProjectVcsDoc::Git),
        name: Some(
            project
                .name
                .clone()
                .unwrap_or_else(|| project.id.to_string()),
        ),
        icon: None,
        commands: ProjectCommandsDoc::default(),
        time: ProjectTimeDoc {
            created: project.created_at.timestamp_millis() as f64,
            updated: project.updated_at.timestamp_millis() as f64,
            initialized: None,
        },
        sandboxes,
    }
}

pub(super) fn compat_session(
    session: &Session,
    project: Option<&Project>,
    meta: &CompatSessionMeta,
) -> SessionDoc {
    let directory = project
        .and_then(|project| project.root.as_ref())
        .map(|root| root.display().to_string())
        .unwrap_or_else(default_directory_string);
    SessionDoc {
        id: compat_session_id(session.id),
        slug: compat_session_id(session.id),
        project_id: session.project_id.to_string(),
        workspace_id: meta.workspace_id.clone(),
        directory,
        parent_id: meta.parent_id.clone(),
        summary: Some(SessionSummaryDoc {
            additions: 0.0,
            deletions: 0.0,
            files: 0.0,
            diffs: Vec::new(),
        }),
        share: meta
            .share_url
            .as_ref()
            .map(|url| SessionShareDoc { url: url.clone() }),
        title: session
            .title
            .clone()
            .unwrap_or_else(|| format!("Session {}", &compat_session_id(session.id)[3..11])),
        version: "v1".to_owned(),
        time: SessionTimeDoc {
            created: session.created_at.timestamp_millis() as f64,
            updated: session.updated_at.timestamp_millis() as f64,
            compacting: None,
            archived: meta.archived_at.map(|value| value as f64),
        },
        permission: serde_json::from_value::<PermissionRuleset>(meta.permission.clone()).ok(),
        revert: None,
    }
}

pub(super) fn compat_global_session(
    session: &Session,
    project: Option<&Project>,
    meta: &CompatSessionMeta,
) -> GlobalSessionDoc {
    let session_doc = compat_session(session, project, meta);
    GlobalSessionDoc {
        id: session_doc.id,
        slug: session_doc.slug,
        project_id: session_doc.project_id,
        workspace_id: session_doc.workspace_id,
        directory: session_doc.directory,
        parent_id: session_doc.parent_id,
        summary: session_doc.summary,
        share: session_doc.share,
        title: session_doc.title,
        version: session_doc.version,
        time: session_doc.time,
        permission: session_doc.permission,
        revert: session_doc.revert,
        project: match project {
            Some(project) => NullableProjectSummaryDoc::Project(ProjectSummaryDoc {
                id: project.id.to_string(),
                name: project.name.clone(),
                worktree: project
                    .root
                    .as_ref()
                    .map(|root| root.display().to_string())
                    .unwrap_or_else(default_directory_string),
            }),
            None => NullableProjectSummaryDoc::Null(()),
        },
    }
}

pub(super) fn compat_message(message: &StoredMessage, parent_id: Option<Ulid>) -> MessageDoc {
    match message.message.role {
        MessageRole::Assistant => MessageDoc::Assistant(AssistantMessageDoc {
            id: compat_message_id(message.id),
            session_id: compat_session_id(message.session_id),
            role: AssistantMessageRoleDoc::Assistant,
            time: MessageTimeCreatedCompletedDoc {
                created: message.created_at.timestamp_millis() as f64,
                completed: Some(message.created_at.timestamp_millis() as f64),
            },
            error: None,
            parent_id: compat_message_id(parent_id.unwrap_or(message.id)),
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
        }),
        _ => MessageDoc::User(UserMessageDoc {
            id: compat_message_id(message.id),
            session_id: compat_session_id(message.session_id),
            role: UserMessageRoleDoc::User,
            time: MessageTimeCreatedDoc {
                created: message.created_at.timestamp_millis() as f64,
            },
            format: None,
            summary: None,
            agent: "general".to_owned(),
            model: MessageModelDoc {
                provider_id: "compat".to_owned(),
                model_id: "compat".to_owned(),
            },
            system: None,
            tools: None,
            variant: None,
        }),
    }
}

pub(super) fn compat_message_with_parts(
    message: &StoredMessage,
    parent_id: Option<Ulid>,
) -> MessageWithPartsDoc {
    MessageWithPartsDoc {
        info: compat_message(message, parent_id),
        parts: compat_parts(message.id, message.session_id, &message.message),
    }
}

pub(super) fn compat_messages_with_parts(messages: &[StoredMessage]) -> Vec<MessageWithPartsDoc> {
    messages
        .iter()
        .enumerate()
        .map(|(index, message)| {
            let parent_id = index.checked_sub(1).map(|parent| messages[parent].id);
            compat_message_with_parts(message, parent_id)
        })
        .collect()
}

pub(super) fn compat_assistant_message_with_parts(
    message: &StoredMessage,
    parent_id: Option<Ulid>,
) -> Option<AssistantMessageWithPartsDoc> {
    match compat_message(message, parent_id) {
        MessageDoc::Assistant(info) => Some(AssistantMessageWithPartsDoc {
            info,
            parts: compat_parts(message.id, message.session_id, &message.message),
        }),
        MessageDoc::User(_) => None,
    }
}

pub(super) fn compat_parts(
    message_id: Ulid,
    session_id: SessionId,
    message: &Message,
) -> Vec<PartDoc> {
    message
        .content
        .iter()
        .enumerate()
        .map(|(index, block)| match block {
            ContentBlock::Text { text }
            | ContentBlock::Reasoning { text }
            | ContentBlock::Refusal { text } => PartDoc::Text(TextPartDoc {
                id: compat_part_id(message_id, index),
                session_id: compat_session_id(session_id),
                message_id: compat_message_id(message_id),
                part_type: TextPartKindDoc::Text,
                text: text.clone(),
                synthetic: None,
                ignored: None,
                time: None,
                metadata: None,
            }),
            ContentBlock::ImageUrl { url } => PartDoc::File(FilePartDoc {
                id: compat_part_id(message_id, index),
                session_id: compat_session_id(session_id),
                message_id: compat_message_id(message_id),
                part_type: FilePartKindDoc::File,
                mime: "application/octet-stream".to_owned(),
                filename: None,
                url: url.clone(),
                source: None,
            }),
            ContentBlock::ToolCall {
                id,
                call_id,
                name,
                input,
            } => PartDoc::Tool(ToolPartDoc {
                id: compat_part_id(message_id, index),
                session_id: compat_session_id(session_id),
                message_id: compat_message_id(message_id),
                part_type: ToolPartKindDoc::Tool,
                call_id: call_id.clone().unwrap_or_else(|| id.clone()),
                tool: name.clone(),
                state: ToolStateDoc::Pending(ToolStatePendingDoc {
                    status: ToolStatePendingKindDoc::Pending,
                    input: json_object(input),
                    raw: json_string(input),
                }),
                metadata: None,
            }),
            ContentBlock::ToolResult {
                call_id,
                output,
                is_error,
            } => PartDoc::Tool(ToolPartDoc {
                id: compat_part_id(message_id, index),
                session_id: compat_session_id(session_id),
                message_id: compat_message_id(message_id),
                part_type: ToolPartKindDoc::Tool,
                call_id: call_id.clone(),
                tool: "tool".to_owned(),
                state: ToolStateDoc::Completed(ToolStateCompletedDoc {
                    status: ToolStateCompletedKindDoc::Completed,
                    input: BTreeMap::new(),
                    output: json_string(output),
                    title: if is_error.unwrap_or(false) {
                        "error".to_owned()
                    } else {
                        "completed".to_owned()
                    },
                    metadata: BTreeMap::new(),
                    time: ToolStateCompletedTimeRangeDoc {
                        start: 0.0,
                        end: 0.0,
                        compacted: None,
                    },
                    attachments: None,
                }),
                metadata: None,
            }),
        })
        .collect()
}
pub(super) fn provider_array(server: &AppState) -> Vec<ProviderDoc> {
    let mut providers = BTreeMap::<String, Vec<crate::types::ProviderModelRecord>>::new();
    for model in server
        .core()
        .list_models()
        .into_iter()
        .map(crate::types::ProviderModelRecord::from)
    {
        providers
            .entry(model.provider_name.clone())
            .or_default()
            .push(model);
    }
    providers
        .into_iter()
        .map(|(name, models)| {
            let models = models
                .into_iter()
                .map(|model| {
                    let model_id = model.model.id.clone();
                    let cost = model.model.cost.clone();
                    let limit = model.model.limit.clone();
                    (
                        model_id.clone(),
                        ModelDoc {
                            id: model_id,
                            provider_id: model.provider_name.clone(),
                            api: ModelApiDoc {
                                id: "compat".to_owned(),
                                url: "https://example.invalid".to_owned(),
                                npm: "compat".to_owned(),
                            },
                            name: model.model.name.clone(),
                            family: model.model.family.clone(),
                            capabilities: ModelCapabilitiesDoc {
                                temperature: model.model.temperature.unwrap_or(false),
                                reasoning: !model.model.reasoning_efforts.is_empty(),
                                attachment: model.model.attachment,
                                toolcall: model.model.tool_call,
                                input: ModelIoDoc {
                                    text: true,
                                    audio: false,
                                    image: false,
                                    video: false,
                                    pdf: false,
                                },
                                output: ModelIoDoc {
                                    text: true,
                                    audio: false,
                                    image: false,
                                    video: false,
                                    pdf: false,
                                },
                                interleaved: ModelInterleavedDoc::Bool(false),
                            },
                            cost: ModelCostDoc {
                                input: cost.as_ref().map(|cost| cost.input).unwrap_or_default(),
                                output: cost.as_ref().map(|cost| cost.output).unwrap_or_default(),
                                cache: MessageTokensCacheDoc {
                                    read: cost
                                        .as_ref()
                                        .and_then(|cost| cost.cache_read)
                                        .unwrap_or_default(),
                                    write: cost
                                        .as_ref()
                                        .and_then(|cost| cost.cache_write)
                                        .unwrap_or_default(),
                                },
                                experimental_over_200k: None,
                            },
                            limit: ModelLimitDoc {
                                context: limit
                                    .as_ref()
                                    .map(|limit| limit.context as f64)
                                    .unwrap_or_default(),
                                input: limit
                                    .as_ref()
                                    .and_then(|limit| limit.input.map(|value| value as f64)),
                                output: limit
                                    .as_ref()
                                    .map(|limit| limit.output as f64)
                                    .unwrap_or_default(),
                            },
                            status: ModelStatusDoc::Active,
                            options: BTreeMap::new(),
                            headers: BTreeMap::new(),
                            release_date: model.model.release_date.clone().unwrap_or_default(),
                            variants: None,
                        },
                    )
                })
                .collect::<BTreeMap<String, ModelDoc>>();
            ProviderDoc {
                id: name.clone(),
                name,
                source: ProviderSourceDoc::Api,
                env: Vec::new(),
                key: None,
                options: BTreeMap::new(),
                models,
            }
        })
        .collect()
}

pub(super) fn provider_default_map(providers: &[ProviderDoc]) -> BTreeMap<String, String> {
    let mut defaults = BTreeMap::new();
    for provider in providers {
        if let Some(model_id) = provider.models.keys().next() {
            defaults.insert(provider.id.clone(), model_id.clone());
        }
    }
    defaults
}
