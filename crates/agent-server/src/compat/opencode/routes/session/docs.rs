use std::collections::BTreeMap;

use aide::openapi::{MediaType, ReferenceOr, RequestBody, SchemaObject, StatusCode};
use aide::transform::{TransformOperation, TransformResponse};
use schemars::{JsonSchema, schema_for};
use serde_json::Value;

use super::super::super::types::errors::{BadRequestErrorDoc, NotFoundErrorDoc};
use super::super::super::*;

pub(super) fn op(id: &'static str) -> impl Fn(TransformOperation) -> TransformOperation + Clone {
    move |op| op.id(id)
}

pub(super) fn op_with_query(
    id: &'static str,
    summary: &'static str,
    description: &'static str,
) -> impl Fn(TransformOperation) -> TransformOperation + Clone {
    move |op| op.id(id).summary(summary).description(description)
}

pub(super) fn bad_request(op: TransformOperation) -> TransformOperation {
    op.response_with::<400, Json<BadRequestErrorDoc>, _>(|res| res.description("Bad request"))
}

pub(super) fn not_found(op: TransformOperation) -> TransformOperation {
    op.response_with::<404, Json<NotFoundErrorDoc>, _>(|res| res.description("Not found"))
}

pub(super) fn json_response<'a, const N: u16, T: JsonSchema>(
    op: TransformOperation<'a>,
    description: &'static str,
) -> TransformOperation<'a> {
    op.response_with::<N, Json<T>, _>(|res: TransformResponse<'_, T>| res.description(description))
}

pub(super) fn inline_json_request<'a, T: JsonSchema>(
    op: TransformOperation<'a>,
    required: bool,
) -> TransformOperation<'a> {
    let schema = inline_schema_value::<T>();
    inline_json_request_from_value(op, required, schema)
}

pub(super) fn session_parameters<'a>(
    op: TransformOperation<'a>,
    names: &[&str],
) -> TransformOperation<'a> {
    parameter_order(op, names)
}

fn parameter_order<'a>(mut op: TransformOperation<'a>, names: &[&str]) -> TransformOperation<'a> {
    let order = names
        .iter()
        .enumerate()
        .map(|(idx, name)| (*name, idx))
        .collect::<BTreeMap<_, _>>();
    op.inner_mut()
        .parameters
        .sort_by_key(|parameter| match parameter {
            ReferenceOr::Item(parameter) => order
                .get(parameter.parameter_data_ref().name.as_str())
                .copied()
                .unwrap_or(usize::MAX),
            ReferenceOr::Reference { .. } => usize::MAX,
        });
    op
}

fn inline_schema_value<T: JsonSchema>() -> Value {
    let mut value =
        serde_json::to_value(schema_for!(T)).expect("serializing schemars schema should succeed");
    if let Some(object) = value.as_object_mut() {
        object.remove("$schema");
        object.remove("title");
    }
    inline_local_defs(&mut value);
    value
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
                if let Some(component_name) = schema.get("title").and_then(Value::as_str) {
                    *value = serde_json::json!({
                        "$ref": format!("#/components/schemas/{component_name}")
                    });
                    return;
                }

                *value = schema.clone();
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

            for child in map.values_mut() {
                inline_local_defs_refs(child, defs);
            }
        }
        Value::Array(items) => {
            for item in items {
                inline_local_defs_refs(item, defs);
            }
        }
        _ => {}
    }
}

fn inline_json_request_from_value<'a>(
    mut op: TransformOperation<'a>,
    required: bool,
    schema: Value,
) -> TransformOperation<'a> {
    op.inner_mut().request_body = Some(ReferenceOr::Item(RequestBody {
        description: None,
        content: [(
            "application/json".to_owned(),
            MediaType {
                schema: Some(SchemaObject {
                    json_schema: serde_json::from_value(schema)
                        .expect("request-body schema should deserialize"),
                    external_docs: None,
                    example: None,
                }),
                ..Default::default()
            },
        )]
        .into_iter()
        .collect(),
        required,
        ..Default::default()
    }));
    if let Some(responses) = op.inner_mut().responses.as_mut() {
        let remove_default_400 = matches!(
            responses
                .responses
                .get(&StatusCode::Code(400))
                .and_then(ReferenceOr::as_item)
                .map(|response| response.description.as_str()),
            Some("Failed to parse the request body as JSON")
        );
        if remove_default_400 {
            responses.responses.shift_remove(&StatusCode::Code(400));
        }
    }
    op
}
