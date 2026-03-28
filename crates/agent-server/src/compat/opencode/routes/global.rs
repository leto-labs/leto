use aide::axum::routing::ApiMethodDocs;
use aide::axum::{
    ApiRouter,
    routing::{get_with, patch_with, post_with},
};
use aide::openapi::{
    Operation, Parameter, ParameterData, ParameterSchemaOrContent, QueryStyle, ReferenceOr,
    SchemaObject, StatusCode,
};
use axum::routing as axum_routing;
use schemars::{JsonSchema, schema_for};
use serde_json::Value;

use super::super::types::common::CompatQuery;
use super::super::types::errors::BadRequestErrorDoc;
use super::super::types::events::{
    EventDoc, EventServerConnectedDoc, EventServerConnectedTypeDoc, GlobalEventDoc,
};
use super::super::types::global::{
    AppLogLevelDoc, AppLogRequestDoc, CompatConfigDoc, EmptyPropertiesDoc, HealthDoc,
    UpgradeRequestDoc, UpgradeResultDoc,
};
use super::super::types::provider::TrueConstDoc;
use super::super::*;

pub(super) fn routes() -> ApiRouter<AppState> {
    ApiRouter::new()
        .merge(global_health_route())
        .merge(global_event_route())
        .merge(global_config_get_route())
        .merge(global_config_patch_route())
        .merge(global_dispose_route())
        .merge(global_upgrade_route())
        .merge(event_subscribe_route())
        .merge(app_log_route())
        .merge(instance_dispose_route())
}

fn global_health_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/global/health",
        get_with(global_health, |operation| {
            operation
                .id("global.health")
                .summary("Get health")
                .description("Get health information about the OpenCode server.")
                .response_with::<200, Json<HealthDoc>, _>(|res| {
                    res.description("Health information")
                })
        }),
    )
}

fn global_event_route() -> ApiRouter<AppState> {
    ApiRouter::new()
        .route("/global/event", axum_routing::get(global_event))
        .api_route_docs(
            "/global/event",
            sse_doc::<GlobalEventDoc>(
                "global.event",
                "Get global events",
                "Subscribe to global events from the OpenCode system using server-sent events.",
                &[],
            ),
        )
}

fn global_config_get_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/global/config",
        get_with(global_config_get, |operation| {
            operation
                .id("global.config.get")
                .summary("Get global configuration")
                .description(
                    "Retrieve the current global OpenCode configuration settings and preferences.",
                )
                .response_with::<200, Json<CompatConfigDoc>, _>(|res| {
                    res.description("Get global config info")
                })
        }),
    )
}

fn global_config_patch_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/global/config",
        patch_with(global_config_patch, |operation| {
            operation
                .id("global.config.update")
                .summary("Update global configuration")
                .description("Update global OpenCode configuration settings and preferences.")
                .response_with::<200, Json<CompatConfigDoc>, _>(|res| {
                    res.description("Successfully updated global config")
                })
                .response_with::<400, Json<BadRequestErrorDoc>, _>(|res| {
                    res.description("Bad request")
                })
        }),
    )
}

fn global_dispose_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/global/dispose",
        post_with(global_dispose, |operation| {
            operation
                .id("global.dispose")
                .summary("Dispose instance")
                .description(
                    "Clean up and dispose all OpenCode instances, releasing all resources.",
                )
                .response_with::<200, Json<bool>, _>(|res| res.description("Global disposed"))
        }),
    )
}

fn global_upgrade_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/global/upgrade",
        post_with(global_upgrade, |operation| {
            operation
                .id("global.upgrade")
                .summary("Upgrade opencode")
                .description(
                    "Upgrade opencode to the specified version or latest if not specified.",
                )
                .response_with::<200, Json<UpgradeResultDoc>, _>(|res| {
                    res.description("Upgrade result")
                })
                .response_with::<400, Json<BadRequestErrorDoc>, _>(|res| {
                    res.description("Bad request")
                })
        }),
    )
}

fn event_subscribe_route() -> ApiRouter<AppState> {
    ApiRouter::new()
        .route("/event", axum_routing::get(global_event))
        .api_route_docs(
            "/event",
            sse_doc::<EventDoc>(
                "event.subscribe",
                "Subscribe to events",
                "Get events",
                &[("directory", None), ("workspace", None)],
            ),
        )
}

fn app_log_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/log",
        post_with(app_log, |operation| {
            operation
                .id("app.log")
                .summary("Write log")
                .description(
                    "Write a log entry to the server logs with specified level and metadata.",
                )
                .response_with::<200, Json<bool>, _>(|res| {
                    res.description("Log entry written successfully")
                })
                .response_with::<400, Json<BadRequestErrorDoc>, _>(|res| {
                    res.description("Bad request")
                })
        }),
    )
}

fn instance_dispose_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/instance/dispose",
        post_with(instance_dispose, |operation| {
            operation
                .id("instance.dispose")
                .summary("Dispose instance")
                .description(
                    "Clean up and dispose the current OpenCode instance, releasing all resources.",
                )
                .response_with::<200, Json<bool>, _>(|res| res.description("Instance disposed"))
        }),
    )
}

fn sse_doc<T: JsonSchema>(
    operation_id: &'static str,
    summary: &'static str,
    description: &'static str,
    query_params: &[(&'static str, Option<&'static str>)],
) -> ApiMethodDocs {
    // `aide` does not infer `Sse<_>` response docs like it does for `Json<T>`,
    // so SSE routes still need a small manual doc shim.
    let mut operation = Operation {
        operation_id: Some(operation_id.to_owned()),
        summary: Some(summary.to_owned()),
        description: Some(description.to_owned()),
        parameters: query_params
            .iter()
            .map(|(name, description)| {
                ReferenceOr::Item(Parameter::Query {
                    parameter_data: ParameterData {
                        name: (*name).to_owned(),
                        description: description.map(|value| value.to_owned()),
                        required: false,
                        deprecated: None,
                        format: ParameterSchemaOrContent::Schema(SchemaObject {
                            json_schema: serde_json::from_value(
                                serde_json::json!({ "type": "string" }),
                            )
                            .expect("query parameter schema should deserialize"),
                            external_docs: None,
                            example: None,
                        }),
                        example: None,
                        examples: Default::default(),
                        explode: None,
                        extensions: Default::default(),
                    },
                    allow_reserved: false,
                    style: QueryStyle::default(),
                    allow_empty_value: None,
                })
            })
            .collect(),
        ..Default::default()
    };
    operation
        .responses
        .get_or_insert_with(Default::default)
        .responses
        .insert(
            StatusCode::Code(200),
            aide::openapi::ReferenceOr::Item(aide::openapi::Response {
                description: "Event stream".to_owned(),
                content: std::iter::once((
                    "text/event-stream".to_owned(),
                    aide::openapi::MediaType {
                        schema: Some(aide::openapi::SchemaObject {
                            // Route-parity tests generate route-local mini-docs without the
                            // full component graph, so the SSE schema must be self-contained.
                            json_schema: serde_json::from_value(inline_sse_schema_value::<T>())
                                .expect("SSE response schema should deserialize"),
                            external_docs: None,
                            example: None,
                        }),
                        ..Default::default()
                    },
                ))
                .collect(),
                ..Default::default()
            }),
        );
    ApiMethodDocs::new("get", operation)
}

fn inline_sse_schema_value<T: JsonSchema>() -> Value {
    let mut value =
        serde_json::to_value(schema_for!(T)).expect("serializing schemars schema should succeed");
    strip_schema_identity_fields(&mut value);
    inline_local_defs(&mut value);
    value
}

fn strip_schema_identity_fields(value: &mut Value) {
    if let Some(object) = value.as_object_mut() {
        object.remove("$schema");
        object.remove("title");
    }
}

fn inline_local_defs(schema: &mut Value) {
    let defs = schema
        .as_object_mut()
        .and_then(|object| object.remove("$defs"))
        .and_then(|defs| defs.as_object().cloned());

    let Some(defs) = defs else {
        return;
    };

    inline_local_defs_refs(schema, &defs);
}

fn inline_local_defs_refs(value: &mut Value, defs: &serde_json::Map<String, Value>) {
    match value {
        Value::Object(map) => {
            if let Some(reference) = map.get("$ref").and_then(Value::as_str)
                && let Some(name) = reference.strip_prefix("#/$defs/")
                && let Some(schema) = defs.get(name)
            {
                *value = schema.clone();
                strip_schema_identity_fields(value);
                inline_local_defs_refs(value, defs);
                return;
            }

            if let Some(local_defs) = map
                .remove("$defs")
                .and_then(|defs| defs.as_object().cloned())
            {
                let mut merged_defs = defs.clone();
                for (name, schema) in local_defs {
                    merged_defs.insert(name, schema);
                }
                for child in map.values_mut() {
                    inline_local_defs_refs(child, &merged_defs);
                }
                return;
            }

            map.remove("$schema");
            for child in map.values_mut() {
                inline_local_defs_refs(child, defs);
            }
        }
        Value::Array(items) => {
            for item in items {
                inline_local_defs_refs(item, defs);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
}

async fn global_health() -> Json<HealthDoc> {
    Json(HealthDoc {
        healthy: TrueConstDoc,
        version: env!("CARGO_PKG_VERSION").to_owned(),
    })
}

async fn global_event(
    State(server): State<AppState>,
) -> Sse<impl futures::Stream<Item = Result<SseEvent, Infallible>>> {
    let connected = stream::once(async {
        let data = serde_json::to_string(&GlobalEventDoc {
            directory: default_directory_string(),
            payload: EventDoc::ServerConnected(EventServerConnectedDoc {
                event_type: EventServerConnectedTypeDoc::Value,
                properties: EmptyPropertiesDoc::default(),
            }),
        })
        .unwrap_or_else(|_| "{}".to_owned());
        Ok::<_, Infallible>(SseEvent::default().data(data))
    });
    let heartbeat =
        tokio_stream::wrappers::IntervalStream::new(tokio::time::interval(Duration::from_secs(10)))
            .map(|_| {
                Ok::<_, Infallible>(
                    SseEvent::default()
                        .data(r#"{"payload":{"type":"server.heartbeat","properties":{}}}"#),
                )
            });
    let events = server.core().subscribe().map(|event| {
        let data = serde_json::to_string(&json!({
            "directory": default_directory_string(),
            "payload": {
                "type": "agent.core_event",
                "properties": event,
            }
        }))
        .unwrap_or_else(|_| "{}".to_owned());
        Ok::<_, Infallible>(SseEvent::default().data(data))
    });
    Sse::new(connected.chain(heartbeat).chain(events))
        .keep_alive(KeepAlive::new().interval(Duration::from_secs(10)))
}

async fn global_config_get(State(server): State<AppState>) -> Response {
    let value = server.compat().global_config.read().await.clone();
    let body = serde_json::from_value::<CompatConfigDoc>(value).unwrap_or_default();
    Json(body).into_response()
}

async fn global_config_patch(
    State(server): State<AppState>,
    Json(body): Json<CompatConfigDoc>,
) -> Response {
    *server.compat().global_config.write().await =
        serde_json::to_value(&body).unwrap_or_else(|_| Value::Object(Default::default()));
    Json(body).into_response()
}

async fn global_dispose(State(server): State<AppState>) -> Response {
    clear_compat_state(&server).await;
    Json(true).into_response()
}

async fn global_upgrade(Json(_body): Json<UpgradeRequestDoc>) -> Response {
    Json(true).into_response()
}

async fn app_log(
    Query(_query): Query<CompatQuery>,
    Json(body): Json<AppLogRequestDoc>,
) -> Response {
    match body.level {
        AppLogLevelDoc::Trace => tracing::trace!(
            target: "agent_server::compat",
            service = %body.service,
            level = ?body.level,
            message = %body.message,
            extra = ?body.extra,
            "compat log"
        ),
        AppLogLevelDoc::Debug => tracing::debug!(
            target: "agent_server::compat",
            service = %body.service,
            level = ?body.level,
            message = %body.message,
            extra = ?body.extra,
            "compat log"
        ),
        AppLogLevelDoc::Info => tracing::info!(
            target: "agent_server::compat",
            service = %body.service,
            level = ?body.level,
            message = %body.message,
            extra = ?body.extra,
            "compat log"
        ),
        AppLogLevelDoc::Warn => tracing::warn!(
            target: "agent_server::compat",
            service = %body.service,
            level = ?body.level,
            message = %body.message,
            extra = ?body.extra,
            "compat log"
        ),
        AppLogLevelDoc::Error => tracing::error!(
            target: "agent_server::compat",
            service = %body.service,
            level = ?body.level,
            message = %body.message,
            extra = ?body.extra,
            "compat log"
        ),
    }
    Json(true).into_response()
}

async fn instance_dispose(
    State(server): State<AppState>,
    Query(_query): Query<CompatQuery>,
) -> Response {
    clear_compat_state(&server).await;
    Json(true).into_response()
}

#[cfg(test)]
mod tests {
    use std::io::{self, Write};
    use std::sync::{Arc, Mutex};

    use crate::{
        compat::opencode::test_utils::{
            normalize_generated_opencode_route_doc, normalize_opencode_route_doc,
            opencode_openapi_options, pinned_opencode_openapi,
        },
        compat::opencode::types::{
            common::CompatQuery,
            global::{AppLogLevelDoc, AppLogRequestDoc},
        },
        utils::openapi::{generate_from_router, subset_for_operations},
    };
    use axum::Json;
    use axum::extract::Query;
    use tracing::subscriber::with_default;

    #[derive(Clone, Default)]
    struct SharedLogBuffer(Arc<Mutex<Vec<u8>>>);

    impl SharedLogBuffer {
        fn contents(&self) -> String {
            String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
        }
    }

    impl Write for SharedLogBuffer {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn global_health_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_opencode_route_doc(&generate_from_router(
                super::global_health_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/global/health", "get")],
            ))
        );
    }

    #[test]
    fn global_event_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_opencode_route_doc(&generate_from_router(
                super::global_event_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/global/event", "get")],
            ))
        );
    }

    #[test]
    fn global_config_get_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_generated_opencode_route_doc(&generate_from_router(
                super::global_config_get_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/global/config", "get")],
            ))
        );
    }

    #[test]
    fn global_config_patch_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_generated_opencode_route_doc(&generate_from_router(
                super::global_config_patch_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/global/config", "patch")],
            ))
        );
    }

    #[test]
    fn global_dispose_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_opencode_route_doc(&generate_from_router(
                super::global_dispose_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/global/dispose", "post")],
            ))
        );
    }

    #[test]
    fn global_upgrade_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_opencode_route_doc(&generate_from_router(
                super::global_upgrade_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/global/upgrade", "post")],
            ))
        );
    }

    #[test]
    fn event_subscribe_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_opencode_route_doc(&generate_from_router(
                super::event_subscribe_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/event", "get")],
            ))
        );
    }

    #[test]
    fn app_log_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_opencode_route_doc(&generate_from_router(
                super::app_log_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/log", "post")],
            ))
        );
    }

    #[test]
    fn app_log_should_emit_compat_log_fields() {
        let log_buffer = SharedLogBuffer::default();
        let subscriber = tracing_subscriber::fmt()
            .with_ansi(false)
            .without_time()
            .with_max_level(tracing::Level::INFO)
            .with_writer({
                let log_buffer = log_buffer.clone();
                move || log_buffer.clone()
            })
            .finish();

        let response = with_default(subscriber, || {
            futures::executor::block_on(super::app_log(
                Query(CompatQuery::default()),
                Json(AppLogRequestDoc {
                    service: "desktop".into(),
                    level: AppLogLevelDoc::Warn,
                    message: "user initiated refresh".into(),
                    extra: Some(std::collections::BTreeMap::from([
                        ("request_id".into(), serde_json::json!("req_123")),
                        ("attempt".into(), serde_json::json!(2)),
                    ])),
                }),
            ))
        });
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let logs = log_buffer.contents();
        assert!(logs.contains("compat log"), "logs were: {logs}");
        assert!(logs.contains("agent_server::compat"), "logs were: {logs}");
        assert!(logs.contains("service=desktop"), "logs were: {logs}");
        assert!(logs.contains("level=Warn"), "logs were: {logs}");
        assert!(logs.contains("user initiated refresh"), "logs were: {logs}");
        assert!(logs.contains("request_id"), "logs were: {logs}");
        assert!(logs.contains("req_123"), "logs were: {logs}");
        assert!(logs.contains("attempt"), "logs were: {logs}");
    }

    #[test]
    fn app_log_should_honor_trace_level_filtering() {
        let suppressed_log_buffer = SharedLogBuffer::default();
        let suppressed_subscriber = tracing_subscriber::fmt()
            .with_ansi(false)
            .without_time()
            .with_max_level(tracing::Level::INFO)
            .with_writer({
                let log_buffer = suppressed_log_buffer.clone();
                move || log_buffer.clone()
            })
            .finish();

        let response = with_default(suppressed_subscriber, || {
            futures::executor::block_on(super::app_log(
                Query(CompatQuery::default()),
                Json(AppLogRequestDoc {
                    service: "desktop".into(),
                    level: AppLogLevelDoc::Trace,
                    message: "trace refresh".into(),
                    extra: Some(std::collections::BTreeMap::from([(
                        "trace_id".into(),
                        serde_json::json!("trace-123"),
                    )])),
                }),
            ))
        });
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        assert!(
            suppressed_log_buffer.contents().trim().is_empty(),
            "trace log should be filtered at INFO: {}",
            suppressed_log_buffer.contents()
        );

        let trace_log_buffer = SharedLogBuffer::default();
        let trace_subscriber = tracing_subscriber::fmt()
            .with_ansi(false)
            .without_time()
            .with_max_level(tracing::Level::TRACE)
            .with_writer({
                let log_buffer = trace_log_buffer.clone();
                move || log_buffer.clone()
            })
            .finish();

        let response = with_default(trace_subscriber, || {
            futures::executor::block_on(super::app_log(
                Query(CompatQuery::default()),
                Json(AppLogRequestDoc {
                    service: "desktop".into(),
                    level: AppLogLevelDoc::Trace,
                    message: "trace refresh".into(),
                    extra: Some(std::collections::BTreeMap::from([(
                        "trace_id".into(),
                        serde_json::json!("trace-123"),
                    )])),
                }),
            ))
        });
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let logs = trace_log_buffer.contents();
        assert!(logs.contains("TRACE"), "logs were: {logs}");
        assert!(logs.contains("compat log"), "logs were: {logs}");
        assert!(logs.contains("agent_server::compat"), "logs were: {logs}");
        assert!(logs.contains("level=Trace"), "logs were: {logs}");
        assert!(logs.contains("trace refresh"), "logs were: {logs}");
        assert!(logs.contains("trace_id"), "logs were: {logs}");
        assert!(logs.contains("trace-123"), "logs were: {logs}");
    }

    #[test]
    fn instance_dispose_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_opencode_route_doc(&generate_from_router(
                super::instance_dispose_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/instance/dispose", "post")],
            ))
        );
    }
}
