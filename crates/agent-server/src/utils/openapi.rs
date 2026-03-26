use std::collections::{BTreeSet, VecDeque};

use aide::axum::ApiRouter;
use aide::openapi::OpenApi;
use aide::transform::TransformOpenApi;
use serde_json::{Map, Value};

const COMPONENT_REF_PREFIX: &str = "#/components/schemas/";

/// Metadata used when generating a small OpenAPI document in tests.
///
/// OpenAPI itself exposes `info.title`, `info.description`, and `info.version`.
/// The top-level `openapi` field is controlled separately via
/// `openapi_version_override`.
#[derive(Debug, Clone, Copy)]
pub(crate) struct OpenApiGenerationOptions<'a> {
    pub title: &'a str,
    pub description: &'a str,
    pub version: &'a str,
    pub openapi_version_override: Option<&'a str>,
}

/// Return a smaller OpenAPI document containing only the requested paths and
/// the transitive `#/components/schemas/*` closure reachable from those paths.
///
/// This helper is intentionally generic:
/// - it knows nothing about OpenCode
/// - it knows nothing about our server or routes
/// - it only understands a common OpenAPI pattern: path items referencing
///   schemas from `components.schemas` via `$ref`
///
/// The output preserves the input document's `openapi` and `info` fields so
/// the returned value is still shaped like a valid mini-spec.
pub(crate) fn subset_for_paths(source: &Value, paths: &[&str]) -> Value {
    let source_paths = source["paths"]
        .as_object()
        .expect("OpenAPI contract should contain paths");

    let mut subset_paths = Map::new();
    let mut component_names = BTreeSet::new();
    let mut pending_refs = VecDeque::new();

    for path in paths {
        let path_item = source_paths
            .get(*path)
            .unwrap_or_else(|| panic!("OpenAPI contract is missing path {path}"));
        collect_schema_refs(path_item, &mut pending_refs);
        subset_paths.insert((*path).to_owned(), path_item.clone());
    }

    let empty_schemas = Map::new();
    let source_schemas = source
        .get("components")
        .and_then(Value::as_object)
        .and_then(|components| components.get("schemas"))
        .and_then(Value::as_object)
        .unwrap_or(&empty_schemas);

    while let Some(reference) = pending_refs.pop_front() {
        let Some(name) = reference.strip_prefix(COMPONENT_REF_PREFIX) else {
            continue;
        };
        if !component_names.insert(name.to_owned()) {
            continue;
        }
        if let Some(schema) = source_schemas.get(name) {
            collect_schema_refs(schema, &mut pending_refs);
        }
    }

    let mut subset_schemas = Map::new();
    for name in component_names {
        subset_schemas.insert(
            name.clone(),
            match source_schemas.get(&name) {
                Some(schema) => schema.clone(),
                None => continue,
            },
        );
    }

    serde_json::json!({
        "openapi": source["openapi"].clone(),
        "info": source["info"].clone(),
        "paths": Value::Object(subset_paths),
        "components": {
            "schemas": Value::Object(subset_schemas),
        }
    })
}

/// Return a smaller OpenAPI document containing only the requested
/// path-operation pairs and the transitive `#/components/schemas/*` closure
/// reachable from those operations.
pub(crate) fn subset_for_operations(source: &Value, operations: &[(&str, &str)]) -> Value {
    let source_paths = source["paths"]
        .as_object()
        .expect("OpenAPI contract should contain paths");

    let mut subset_paths = Map::new();
    let mut component_names = BTreeSet::new();
    let mut pending_refs = VecDeque::new();

    for (path, method) in operations {
        let path_item = source_paths
            .get(*path)
            .unwrap_or_else(|| panic!("OpenAPI contract is missing path {path}"));
        let operation = path_item
            .as_object()
            .and_then(|path_item| path_item.get(*method))
            .unwrap_or_else(|| {
                panic!("OpenAPI contract is missing operation {method} for path {path}")
            });

        collect_schema_refs(operation, &mut pending_refs);

        subset_paths
            .entry((*path).to_owned())
            .or_insert_with(|| Value::Object(Map::new()))
            .as_object_mut()
            .expect("subset path item should be an object")
            .insert((*method).to_owned(), operation.clone());
    }

    let empty_schemas = Map::new();
    let source_schemas = source
        .get("components")
        .and_then(Value::as_object)
        .and_then(|components| components.get("schemas"))
        .and_then(Value::as_object)
        .unwrap_or(&empty_schemas);

    while let Some(reference) = pending_refs.pop_front() {
        let Some(name) = reference.strip_prefix(COMPONENT_REF_PREFIX) else {
            continue;
        };
        if !component_names.insert(name.to_owned()) {
            continue;
        }
        if let Some(schema) = source_schemas.get(name) {
            collect_schema_refs(schema, &mut pending_refs);
        }
    }

    let mut subset_schemas = Map::new();
    for name in component_names {
        subset_schemas.insert(
            name.clone(),
            match source_schemas.get(&name) {
                Some(schema) => schema.clone(),
                None => continue,
            },
        );
    }

    serde_json::json!({
        "openapi": source["openapi"].clone(),
        "info": source["info"].clone(),
        "paths": Value::Object(subset_paths),
        "components": {
            "schemas": Value::Object(subset_schemas),
        }
    })
}

/// Return a copy of an OpenAPI document with the given numeric response codes
/// removed from every operation under `paths`.
///
/// This is intentionally a small, generic utility for tests that want to
/// ignore known-extra generator responses without introducing route- or
/// product-specific logic.
pub(crate) fn exclude_response_codes(source: &Value, codes: &[u16]) -> Value {
    let excluded = codes
        .iter()
        .map(|code| code.to_string())
        .collect::<BTreeSet<_>>();
    let mut doc = source.clone();

    for_each_operation_mut(&mut doc, |operation| {
        let Some(responses) = operation
            .get_mut("responses")
            .and_then(Value::as_object_mut)
        else {
            return;
        };
        responses.retain(|status_code, _| !excluded.contains(status_code));
    });

    doc
}

/// Return a copy of an OpenAPI document with the named component schemas
/// removed from `components.schemas`.
///
/// This is a small manual escape hatch for focused tests that want to ignore a
/// known schema by name without changing the rest of the document.
pub(crate) fn exclude_schemas(source: &Value, names: &[&str]) -> Value {
    let excluded = names.iter().copied().collect::<BTreeSet<_>>();
    let mut doc = source.clone();

    let Some(schemas) = doc
        .get_mut("components")
        .and_then(Value::as_object_mut)
        .and_then(|components| components.get_mut("schemas"))
        .and_then(Value::as_object_mut)
    else {
        return doc;
    };

    schemas.retain(|name, _| !excluded.contains(name.as_str()));
    doc
}

/// Return a copy of an OpenAPI document with any unreferenced
/// `components.schemas` entries removed.
///
/// Reachability is computed from the document's current `paths` section and
/// then expanded transitively through nested `#/components/schemas/*` refs.
pub(crate) fn exclude_unreferenced_schemas(source: &Value) -> Value {
    let Some(source_paths) = source.get("paths").and_then(Value::as_object) else {
        return source.clone();
    };

    let Some(source_schemas) = source
        .get("components")
        .and_then(Value::as_object)
        .and_then(|components| components.get("schemas"))
        .and_then(Value::as_object)
    else {
        return source.clone();
    };

    let referenced_names = collect_referenced_schema_names(source_paths.values(), source_schemas);

    let mut doc = source.clone();
    let Some(schemas) = doc
        .get_mut("components")
        .and_then(Value::as_object_mut)
        .and_then(|components| components.get_mut("schemas"))
        .and_then(Value::as_object_mut)
    else {
        return doc;
    };

    schemas.retain(|name, _| referenced_names.contains(name));
    doc
}

/// Return a copy of an OpenAPI document with selected relative fields removed
/// from every parameter object.
///
/// Relative paths are slash-separated from the parameter object root, for
/// example:
/// - `style`
/// - `schema/default`
pub(crate) fn exclude_parameter_fields(source: &Value, field_paths: &[&str]) -> Value {
    let mut doc = source.clone();

    for_each_parameter_mut(&mut doc, |parameter| {
        for field_path in field_paths {
            remove_relative_path_from_object(parameter, field_path);
        }
    });

    doc
}

/// Return a copy of an OpenAPI document with top-level `description` fields
/// removed from every parameter object.
///
/// This is useful for parity tests where human-readable parameter prose is not
/// contract-significant and the pinned source is inconsistent about including
/// it.
pub(crate) fn exclude_parameter_descriptions(source: &Value) -> Value {
    let mut doc = source.clone();

    for_each_parameter_mut(&mut doc, |parameter| {
        parameter.remove("description");
    });

    doc
}

/// Return a copy of an OpenAPI document with selected relative fields removed
/// from every request body object.
///
/// Relative paths are slash-separated from the request body object root, for
/// example:
/// - `required`
pub(crate) fn exclude_request_body_fields(source: &Value, field_paths: &[&str]) -> Value {
    let mut doc = source.clone();

    for_each_request_body_mut(&mut doc, |request_body| {
        for field_path in field_paths {
            remove_relative_path_from_object(request_body, field_path);
        }
    });

    doc
}

/// Return a copy of an OpenAPI document with every operation's `parameters`
/// array sorted by a stable key.
///
/// This reduces low-value parity diffs when generators choose a different
/// insertion order for otherwise identical parameter objects.
pub(crate) fn sort_parameters(source: &Value) -> Value {
    let mut doc = source.clone();

    for_each_operation_mut(&mut doc, |operation| {
        let Some(parameters) = operation
            .get_mut("parameters")
            .and_then(Value::as_array_mut)
        else {
            return;
        };
        parameters.sort_by_cached_key(parameter_sort_key);
    });

    doc
}

/// Return a copy of an OpenAPI document where schema nodes with a single-value
/// `enum` are rewritten to use `const` instead.
///
/// This keeps the schema meaning the same while reducing low-value
/// representation diffs between generators that choose `enum: [x]` and specs
/// that choose `const: x`.
pub(crate) fn normalize_single_value_enums_to_const(source: &Value) -> Value {
    let mut doc = source.clone();

    apply_schema_visit(
        &mut doc,
        SchemaRootTargets::ALL,
        SchemaVisitOptions {
            field_paths: &[],
            normalize_single_value_enum: true,
            normalize_true_schema: false,
            exclude_numeric_format: false,
            normalize_integral_default: false,
        },
    );

    doc
}

/// Return a copy of an OpenAPI document where schema nodes equal to the
/// boolean schema `true` are rewritten to `{}`.
///
/// This keeps the schema meaning the same while reducing low-value
/// representation diffs between generators that choose boolean-`true` schemas
/// and specs that choose empty object schemas.
pub(crate) fn normalize_true_schemas_to_empty_objects(source: &Value) -> Value {
    let mut doc = source.clone();

    apply_schema_visit(
        &mut doc,
        SchemaRootTargets::ALL,
        SchemaVisitOptions {
            field_paths: &[],
            normalize_single_value_enum: false,
            normalize_true_schema: true,
            exclude_numeric_format: false,
            normalize_integral_default: false,
        },
    );

    doc
}

/// Return a copy of an OpenAPI document where numeric schema nodes omit
/// `format`.
///
/// This keeps the schema meaning used in our pinned compat tests while
/// reducing low-value diffs such as `number` + `double` and `integer` +
/// `int64`.
pub(crate) fn exclude_numeric_schema_formats(source: &Value) -> Value {
    let mut doc = source.clone();

    apply_schema_visit(
        &mut doc,
        SchemaRootTargets::ALL,
        SchemaVisitOptions {
            field_paths: &[],
            normalize_single_value_enum: false,
            normalize_true_schema: false,
            exclude_numeric_format: true,
            normalize_integral_default: false,
        },
    );

    doc
}

/// Return a copy of an OpenAPI document where schema-node `default` values
/// that are integral JSON numbers use integer notation.
///
/// This reduces low-value diffs such as `5000.0` vs `5000` while staying
/// scoped to schema defaults rather than arbitrary numeric values.
pub(crate) fn normalize_integral_schema_defaults(source: &Value) -> Value {
    let mut doc = source.clone();

    apply_schema_visit(
        &mut doc,
        SchemaRootTargets::ALL,
        SchemaVisitOptions {
            field_paths: &[],
            normalize_single_value_enum: false,
            normalize_true_schema: false,
            exclude_numeric_format: false,
            normalize_integral_default: true,
        },
    );

    doc
}

/// Return a copy of an OpenAPI document with JSON Schema `default` metadata
/// removed from response body schemas only.
///
/// This intentionally does not touch:
/// - request body schemas
/// - parameter schemas
/// - OpenAPI `responses.default`
/// - payload properties literally named `"default"`
pub(crate) fn exclude_response_schema_defaults(source: &Value) -> Value {
    let mut doc = source.clone();

    apply_schema_visit(
        &mut doc,
        SchemaRootTargets::RESPONSES_ONLY,
        SchemaVisitOptions {
            field_paths: &["default"],
            normalize_single_value_enum: false,
            normalize_true_schema: false,
            exclude_numeric_format: false,
            normalize_integral_default: false,
        },
    );

    doc
}

/// Return a copy of an OpenAPI document where non-required object properties
/// with simple nullable schemas are rewritten to their non-null form.
///
/// This intentionally targets only a narrow class of schemas that commonly
/// arise from `Option<T>` in generators:
/// - `type: [X, "null"]`
/// - `anyOf: [X, { "type": "null" }]`
///
/// It only applies to object properties that are not listed in the parent
/// schema's `required` array. Required properties and arbitrary nullable schema
/// nodes elsewhere in the document are left untouched.
pub(crate) fn normalize_optional_property_nullability(source: &Value) -> Value {
    let mut doc = source.clone();
    for_each_schema_root_mut(&mut doc, SchemaRootTargets::ALL, |schema| {
        normalize_optional_property_nullability_schema(schema);
    });

    doc
}

/// Return a copy of an OpenAPI document where optional parameter schemas with
/// simple nullable forms are rewritten to their non-null form.
///
/// This intentionally targets only operation parameters where `required` is
/// not `true`, leaving required parameters and non-parameter schemas
/// untouched.
pub(crate) fn normalize_optional_parameter_nullability(source: &Value) -> Value {
    let mut doc = source.clone();
    for_each_parameter_mut(&mut doc, |parameter| {
        if parameter
            .get("required")
            .and_then(Value::as_bool)
            .is_some_and(|required| required)
        {
            return;
        }
        if let Some(schema) = parameter.get_mut("schema") {
            normalize_simple_nullable_property_schema(schema);
        }
    });

    doc
}

/// Return a copy of an OpenAPI document where integer schemas that use
/// inclusive lower bounds are rewritten to the equivalent exclusive lower
/// bound form.
///
/// For integer-valued schemas, `minimum: N` is equivalent to
/// `exclusiveMinimum: N - 1`. This reduces low-value parity diffs against
/// pinned specs that prefer the exclusive form.
pub(crate) fn normalize_integer_minimum_equivalents(source: &Value) -> Value {
    let mut doc = source.clone();
    for_each_schema_root_mut(&mut doc, SchemaRootTargets::ALL, |schema| {
        normalize_integer_minimum_equivalents_schema(schema);
    });

    doc
}

/// Return a copy of an OpenAPI document where schema nodes that wrap a single
/// schema under `allOf` are flattened into that schema plus the wrapper's
/// sibling metadata.
///
/// This is intentionally narrow: it only rewrites the safe one-entry case and
/// leaves general `allOf` composition untouched.
pub(crate) fn flatten_single_entry_all_of(source: &Value) -> Value {
    let mut doc = source.clone();
    for_each_schema_root_mut(&mut doc, SchemaRootTargets::ALL, |schema| {
        flatten_single_entry_all_of_schema(schema);
    });

    doc
}

/// Return a copy of an OpenAPI document where an existing top-level
/// `components` object always contains a `schemas` object.
pub(crate) fn normalize_empty_components_schemas(source: &Value) -> Value {
    let mut doc = source.clone();

    let Some(components) = doc.get_mut("components").and_then(Value::as_object_mut) else {
        return doc;
    };

    components
        .entry("schemas".to_owned())
        .or_insert_with(|| Value::Object(Map::new()));
    doc
}

/// Return a copy of an OpenAPI document with selected relative metadata fields
/// removed from every discovered schema node.
///
/// Relative paths are slash-separated from each schema node root, for example:
/// - `additionalProperties`
/// - `propertyNames`
///
/// This helper recursively discovers schema nodes in schema-bearing locations
/// such as `components.schemas`, parameter schemas, request body schemas, and
/// response body schemas. It applies the removals to each schema node root, but
/// does not recursively delete matching keys from arbitrary descendants.
pub(crate) fn exclude_schema_metadata_fields(source: &Value, field_paths: &[&str]) -> Value {
    let mut doc = source.clone();

    apply_schema_visit(
        &mut doc,
        SchemaRootTargets::ALL,
        SchemaVisitOptions {
            field_paths,
            normalize_single_value_enum: false,
            normalize_true_schema: false,
            exclude_numeric_format: false,
            normalize_integral_default: false,
        },
    );

    doc
}

/// Return a copy of an OpenAPI document where schema nodes that are pure
/// `#/components/schemas/*` refs are replaced with the referenced schema.
///
/// This is useful in route-level parity tests when one side chooses a reusable
/// component ref and the other side inlines the same schema.
pub(crate) fn expand_schema_refs(source: &Value) -> Value {
    let Some(source_schemas) = source
        .get("components")
        .and_then(Value::as_object)
        .and_then(|components| components.get("schemas"))
        .and_then(Value::as_object)
    else {
        return source.clone();
    };

    let mut doc = source.clone();

    for_each_schema_root_mut(&mut doc, SchemaRootTargets::ALL, |schema| {
        expand_schema_node(schema, source_schemas, &mut Vec::new());
    });

    doc
}

/// Return a copy of an OpenAPI document where schema refs are expanded only
/// for the selected component schema names.
///
/// This is useful when a later normalization step intentionally drops helper
/// components and route-parity tests need those helper schemas inlined first,
/// without aggressively inlining the entire component graph.
pub(crate) fn expand_selected_schema_refs(source: &Value, names: &[&str]) -> Value {
    let Some(source_schemas) = source
        .get("components")
        .and_then(Value::as_object)
        .and_then(|components| components.get("schemas"))
        .and_then(Value::as_object)
    else {
        return source.clone();
    };

    let selected = names.iter().copied().collect::<BTreeSet<_>>();
    let mut doc = source.clone();

    for_each_schema_root_mut(&mut doc, SchemaRootTargets::ALL, |schema| {
        expand_selected_schema_node(schema, source_schemas, &selected, &mut Vec::new());
    });

    doc
}

/// Run `aide` OpenAPI generation for an `ApiRouter` and return the serialized
/// document as JSON.
///
/// This helper intentionally performs only the base `aide` generation step. It
/// does not apply any of our compat-specific post-processing such as path
/// prefixing, component registration, or normalization. That makes it useful
/// for small route-family tests that want to study what `aide` emits on its own.
///
/// The router is built inside this helper, after the `aide::generate`
/// thread-local context has been configured. `aide` records schema references
/// while routes are being constructed, so accepting a fully-built `ApiRouter`
/// here would reset the schema generator too late and drop the collected
/// definitions before `finish_api_with(...)` can move them into
/// `components.schemas`.
pub(crate) fn generate_from_router<S, F>(
    build_router: F,
    options: OpenApiGenerationOptions<'_>,
) -> Value
where
    S: Clone + Send + Sync + 'static,
    F: FnOnce() -> ApiRouter<S>,
{
    aide::generate::reset_context();
    aide::generate::on_error(|error| {
        tracing::warn!(?error, "aide failed while generating test OpenAPI");
    });
    aide::generate::extract_schemas(true);
    aide::generate::infer_responses(false);

    let router = build_router();
    let mut openapi = OpenApi::default();
    let _ = router.finish_api_with(&mut openapi, |api: TransformOpenApi<'_>| {
        api.title(options.title)
            .description(options.description)
            .version(options.version)
    });

    let mut doc = serde_json::to_value(openapi)
        .unwrap_or_else(|_| serde_json::json!({"openapi": "3.1.1", "paths": {}}));
    if let Some(version) = options.openapi_version_override {
        doc["openapi"] = Value::String(version.to_owned());
    }
    doc
}

fn collect_referenced_schema_names<'a>(
    roots: impl IntoIterator<Item = &'a Value>,
    source_schemas: &Map<String, Value>,
) -> BTreeSet<String> {
    let mut component_names = BTreeSet::new();
    let mut pending_refs = VecDeque::new();

    for root in roots {
        collect_schema_refs(root, &mut pending_refs);
    }

    while let Some(reference) = pending_refs.pop_front() {
        let Some(name) = reference.strip_prefix(COMPONENT_REF_PREFIX) else {
            continue;
        };
        if !component_names.insert(name.to_owned()) {
            continue;
        }
        if let Some(schema) = source_schemas.get(name) {
            collect_schema_refs(schema, &mut pending_refs);
        }
    }

    component_names
}

fn remove_relative_path_from_object(root: &mut Map<String, Value>, field_path: &str) {
    let mut segments = field_path.split('/').filter(|segment| !segment.is_empty());
    let Some(first) = segments.next() else {
        return;
    };

    let mut current = root;
    let mut segment = first;

    for next in segments {
        let Some(child) = current.get_mut(segment).and_then(Value::as_object_mut) else {
            return;
        };
        current = child;
        segment = next;
    }

    current.remove(segment);
}

#[derive(Clone, Copy)]
struct SchemaRootTargets {
    components: bool,
    parameters: bool,
    request_bodies: bool,
    responses: bool,
}

impl SchemaRootTargets {
    const ALL: Self = Self {
        components: true,
        parameters: true,
        request_bodies: true,
        responses: true,
    };

    const RESPONSES_ONLY: Self = Self {
        components: false,
        parameters: false,
        request_bodies: false,
        responses: true,
    };
}

#[derive(Clone, Copy)]
struct SchemaVisitOptions<'a> {
    field_paths: &'a [&'a str],
    normalize_single_value_enum: bool,
    normalize_true_schema: bool,
    exclude_numeric_format: bool,
    normalize_integral_default: bool,
}

fn for_each_operation_mut(doc: &mut Value, mut visit: impl FnMut(&mut Map<String, Value>)) {
    let Some(paths) = doc.get_mut("paths").and_then(Value::as_object_mut) else {
        return;
    };

    for path_item in paths.values_mut() {
        let Some(operations) = path_item.as_object_mut() else {
            continue;
        };

        for operation in operations.values_mut() {
            let Some(operation) = operation.as_object_mut() else {
                continue;
            };
            visit(operation);
        }
    }
}

fn for_each_parameter_mut(doc: &mut Value, mut visit: impl FnMut(&mut Map<String, Value>)) {
    for_each_operation_mut(doc, |operation| {
        let Some(parameters) = operation
            .get_mut("parameters")
            .and_then(Value::as_array_mut)
        else {
            return;
        };

        for parameter in parameters {
            let Some(parameter) = parameter.as_object_mut() else {
                continue;
            };
            visit(parameter);
        }
    });
}

fn for_each_request_body_mut(doc: &mut Value, mut visit: impl FnMut(&mut Map<String, Value>)) {
    for_each_operation_mut(doc, |operation| {
        let Some(request_body) = operation
            .get_mut("requestBody")
            .and_then(Value::as_object_mut)
        else {
            return;
        };
        visit(request_body);
    });
}

fn for_each_response_mut(doc: &mut Value, mut visit: impl FnMut(&mut Map<String, Value>)) {
    for_each_operation_mut(doc, |operation| {
        let Some(responses) = operation
            .get_mut("responses")
            .and_then(Value::as_object_mut)
        else {
            return;
        };

        for response in responses.values_mut() {
            let Some(response) = response.as_object_mut() else {
                continue;
            };
            visit(response);
        }
    });
}

fn for_each_schema_root_mut(
    doc: &mut Value,
    targets: SchemaRootTargets,
    mut visit: impl FnMut(&mut Value),
) {
    if targets.components
        && let Some(schemas) = doc
            .get_mut("components")
            .and_then(Value::as_object_mut)
            .and_then(|components| components.get_mut("schemas"))
            .and_then(Value::as_object_mut)
    {
        for schema in schemas.values_mut() {
            visit(schema);
        }
    }

    if targets.parameters {
        for_each_parameter_mut(doc, |parameter| {
            if let Some(schema) = parameter.get_mut("schema") {
                visit(schema);
            }
        });
    }

    if targets.request_bodies {
        for_each_request_body_mut(doc, |request_body| {
            let Some(content) = request_body
                .get_mut("content")
                .and_then(Value::as_object_mut)
            else {
                return;
            };

            for media_type in content.values_mut() {
                if let Some(schema) = media_type
                    .as_object_mut()
                    .and_then(|media_type| media_type.get_mut("schema"))
                {
                    visit(schema);
                }
            }
        });
    }

    if targets.responses {
        for_each_response_mut(doc, |response| {
            let Some(content) = response.get_mut("content").and_then(Value::as_object_mut) else {
                return;
            };

            for media_type in content.values_mut() {
                if let Some(schema) = media_type
                    .as_object_mut()
                    .and_then(|media_type| media_type.get_mut("schema"))
                {
                    visit(schema);
                }
            }
        });
    }
}

fn apply_schema_visit(
    doc: &mut Value,
    targets: SchemaRootTargets,
    options: SchemaVisitOptions<'_>,
) {
    for_each_schema_root_mut(doc, targets, |schema| visit_schema_nodes(schema, options));
}

fn visit_schema_nodes(schema: &mut Value, options: SchemaVisitOptions<'_>) {
    if options.normalize_true_schema && matches!(schema, Value::Bool(true)) {
        *schema = Value::Object(Map::new());
        return;
    }

    let Some(schema_object) = schema.as_object_mut() else {
        return;
    };

    if options.exclude_numeric_format && schema_type_is_numeric(schema_object.get("type")) {
        schema_object.remove("format");
    }
    if options.normalize_integral_default {
        normalize_schema_default_number(schema_object);
    }

    if options.normalize_single_value_enum
        && !schema_object.contains_key("const")
        && let Some(Value::Array(values)) = schema_object.get("enum")
        && values.len() == 1
    {
        let value = values[0].clone();
        schema_object.remove("enum");
        schema_object.insert("const".to_owned(), value);
    }

    for field_path in options.field_paths {
        remove_relative_path_from_object(schema_object, field_path);
    }

    if let Some(properties) = schema_object
        .get_mut("properties")
        .and_then(Value::as_object_mut)
    {
        for child in properties.values_mut() {
            visit_schema_nodes(child, options);
        }
    }

    if let Some(pattern_properties) = schema_object
        .get_mut("patternProperties")
        .and_then(Value::as_object_mut)
    {
        for child in pattern_properties.values_mut() {
            visit_schema_nodes(child, options);
        }
    }

    if let Some(definitions) = schema_object
        .get_mut("$defs")
        .and_then(Value::as_object_mut)
    {
        for child in definitions.values_mut() {
            visit_schema_nodes(child, options);
        }
    }

    if let Some(definitions) = schema_object
        .get_mut("definitions")
        .and_then(Value::as_object_mut)
    {
        for child in definitions.values_mut() {
            visit_schema_nodes(child, options);
        }
    }

    visit_optional_schema_child(schema_object.get_mut("items"), options);
    visit_optional_schema_child(schema_object.get_mut("contains"), options);
    visit_optional_schema_child(schema_object.get_mut("not"), options);
    visit_optional_schema_child(schema_object.get_mut("if"), options);
    visit_optional_schema_child(schema_object.get_mut("then"), options);
    visit_optional_schema_child(schema_object.get_mut("else"), options);
    visit_optional_schema_child(schema_object.get_mut("additionalProperties"), options);
    visit_optional_schema_child(schema_object.get_mut("propertyNames"), options);
    visit_optional_schema_child(schema_object.get_mut("unevaluatedItems"), options);
    visit_optional_schema_child(schema_object.get_mut("unevaluatedProperties"), options);

    if let Some(children) = schema_object
        .get_mut("prefixItems")
        .and_then(Value::as_array_mut)
    {
        for child in children {
            visit_schema_nodes(child, options);
        }
    }

    for keyword in ["allOf", "anyOf", "oneOf"] {
        if let Some(children) = schema_object.get_mut(keyword).and_then(Value::as_array_mut) {
            for child in children {
                visit_schema_nodes(child, options);
            }
        }
    }

    if let Some(children) = schema_object
        .get_mut("dependentSchemas")
        .and_then(Value::as_object_mut)
    {
        for child in children.values_mut() {
            visit_schema_nodes(child, options);
        }
    }
}

fn visit_optional_schema_child(child: Option<&mut Value>, options: SchemaVisitOptions<'_>) {
    let Some(child) = child else {
        return;
    };

    match child {
        Value::Object(_) | Value::Bool(true) => visit_schema_nodes(child, options),
        Value::Array(children) => {
            for child in children {
                visit_schema_nodes(child, options);
            }
        }
        _ => {}
    }
}

fn parameter_sort_key(parameter: &Value) -> (String, String) {
    let Some(parameter) = parameter.as_object() else {
        return (String::new(), String::new());
    };

    let location = parameter
        .get("in")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let name = parameter
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();

    (location, name)
}

fn schema_type_is_numeric(schema_type: Option<&Value>) -> bool {
    match schema_type {
        Some(Value::String(kind)) => matches!(kind.as_str(), "number" | "integer"),
        Some(Value::Array(kinds)) => kinds.iter().any(|kind| {
            kind.as_str()
                .is_some_and(|kind| matches!(kind, "number" | "integer"))
        }),
        _ => false,
    }
}

fn normalize_schema_default_number(schema_object: &mut Map<String, Value>) {
    let Some(default) = schema_object.get_mut("default") else {
        return;
    };

    let Some(value) = default.as_f64() else {
        return;
    };
    if !value.is_finite() || value.fract() != 0.0 {
        return;
    }

    if value >= 0.0 && value <= u64::MAX as f64 {
        *default = Value::Number((value as u64).into());
    } else if value >= i64::MIN as f64 && value <= i64::MAX as f64 {
        *default = Value::Number((value as i64).into());
    }
}

fn flatten_single_entry_all_of_schema(schema: &mut Value) {
    let Some(schema_object) = schema.as_object_mut() else {
        return;
    };

    if let Some(all_of) = schema_object.remove("allOf") {
        match all_of {
            Value::Array(children) if children.len() == 1 => {
                if let Some(child_object) = children.first().and_then(Value::as_object).cloned() {
                    for (key, value) in child_object {
                        schema_object.entry(key).or_insert(value);
                    }
                } else {
                    schema_object.insert("allOf".to_owned(), Value::Array(children));
                }
            }
            other => {
                schema_object.insert("allOf".to_owned(), other);
            }
        }
    }

    if let Some(properties) = schema_object
        .get_mut("properties")
        .and_then(Value::as_object_mut)
    {
        for child in properties.values_mut() {
            flatten_single_entry_all_of_schema(child);
        }
    }

    if let Some(pattern_properties) = schema_object
        .get_mut("patternProperties")
        .and_then(Value::as_object_mut)
    {
        for child in pattern_properties.values_mut() {
            flatten_single_entry_all_of_schema(child);
        }
    }

    if let Some(definitions) = schema_object
        .get_mut("$defs")
        .and_then(Value::as_object_mut)
    {
        for child in definitions.values_mut() {
            flatten_single_entry_all_of_schema(child);
        }
    }

    if let Some(definitions) = schema_object
        .get_mut("definitions")
        .and_then(Value::as_object_mut)
    {
        for child in definitions.values_mut() {
            flatten_single_entry_all_of_schema(child);
        }
    }

    flatten_optional_schema_child(schema_object.get_mut("items"));
    flatten_optional_schema_child(schema_object.get_mut("contains"));
    flatten_optional_schema_child(schema_object.get_mut("not"));
    flatten_optional_schema_child(schema_object.get_mut("if"));
    flatten_optional_schema_child(schema_object.get_mut("then"));
    flatten_optional_schema_child(schema_object.get_mut("else"));
    flatten_optional_schema_child(schema_object.get_mut("additionalProperties"));
    flatten_optional_schema_child(schema_object.get_mut("propertyNames"));
    flatten_optional_schema_child(schema_object.get_mut("unevaluatedItems"));
    flatten_optional_schema_child(schema_object.get_mut("unevaluatedProperties"));

    if let Some(children) = schema_object
        .get_mut("prefixItems")
        .and_then(Value::as_array_mut)
    {
        for child in children {
            flatten_single_entry_all_of_schema(child);
        }
    }

    for keyword in ["allOf", "anyOf", "oneOf"] {
        if let Some(children) = schema_object.get_mut(keyword).and_then(Value::as_array_mut) {
            for child in children {
                flatten_single_entry_all_of_schema(child);
            }
        }
    }

    if let Some(children) = schema_object
        .get_mut("dependentSchemas")
        .and_then(Value::as_object_mut)
    {
        for child in children.values_mut() {
            flatten_single_entry_all_of_schema(child);
        }
    }
}

fn flatten_optional_schema_child(child: Option<&mut Value>) {
    let Some(child) = child else {
        return;
    };

    match child {
        Value::Object(_) | Value::Bool(true) => flatten_single_entry_all_of_schema(child),
        Value::Array(children) => {
            for child in children {
                flatten_single_entry_all_of_schema(child);
            }
        }
        _ => {}
    }
}

fn normalize_optional_property_nullability_schema(schema: &mut Value) {
    let Some(schema_object) = schema.as_object_mut() else {
        return;
    };

    let required = schema_object
        .get("required")
        .and_then(Value::as_array)
        .map(|required| {
            required
                .iter()
                .filter_map(Value::as_str)
                .map(ToOwned::to_owned)
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_default();

    if let Some(properties) = schema_object
        .get_mut("properties")
        .and_then(Value::as_object_mut)
    {
        for (name, child) in properties.iter_mut() {
            if !required.contains(name) {
                normalize_simple_nullable_property_schema(child);
            }
            normalize_optional_property_nullability_schema(child);
        }
    }

    if let Some(pattern_properties) = schema_object
        .get_mut("patternProperties")
        .and_then(Value::as_object_mut)
    {
        for child in pattern_properties.values_mut() {
            normalize_optional_property_nullability_schema(child);
        }
    }

    if let Some(definitions) = schema_object
        .get_mut("$defs")
        .and_then(Value::as_object_mut)
    {
        for child in definitions.values_mut() {
            normalize_optional_property_nullability_schema(child);
        }
    }

    if let Some(definitions) = schema_object
        .get_mut("definitions")
        .and_then(Value::as_object_mut)
    {
        for child in definitions.values_mut() {
            normalize_optional_property_nullability_schema(child);
        }
    }

    normalize_optional_property_nullability_child(schema_object.get_mut("items"));
    normalize_optional_property_nullability_child(schema_object.get_mut("contains"));
    normalize_optional_property_nullability_child(schema_object.get_mut("not"));
    normalize_optional_property_nullability_child(schema_object.get_mut("if"));
    normalize_optional_property_nullability_child(schema_object.get_mut("then"));
    normalize_optional_property_nullability_child(schema_object.get_mut("else"));
    normalize_optional_property_nullability_child(schema_object.get_mut("additionalProperties"));
    normalize_optional_property_nullability_child(schema_object.get_mut("propertyNames"));
    normalize_optional_property_nullability_child(schema_object.get_mut("unevaluatedItems"));
    normalize_optional_property_nullability_child(schema_object.get_mut("unevaluatedProperties"));

    if let Some(children) = schema_object
        .get_mut("prefixItems")
        .and_then(Value::as_array_mut)
    {
        for child in children {
            normalize_optional_property_nullability_schema(child);
        }
    }

    for keyword in ["allOf", "anyOf", "oneOf"] {
        if let Some(children) = schema_object.get_mut(keyword).and_then(Value::as_array_mut) {
            for child in children {
                normalize_optional_property_nullability_schema(child);
            }
        }
    }

    if let Some(children) = schema_object
        .get_mut("dependentSchemas")
        .and_then(Value::as_object_mut)
    {
        for child in children.values_mut() {
            normalize_optional_property_nullability_schema(child);
        }
    }
}

fn normalize_optional_property_nullability_child(child: Option<&mut Value>) {
    let Some(child) = child else {
        return;
    };

    match child {
        Value::Object(_) | Value::Bool(true) => {
            normalize_optional_property_nullability_schema(child)
        }
        Value::Array(children) => {
            for child in children {
                normalize_optional_property_nullability_schema(child);
            }
        }
        _ => {}
    }
}

fn normalize_integer_minimum_equivalents_schema(schema: &mut Value) {
    let Some(schema_object) = schema.as_object_mut() else {
        return;
    };

    normalize_integer_minimum_equivalent(schema_object);

    if let Some(properties) = schema_object
        .get_mut("properties")
        .and_then(Value::as_object_mut)
    {
        for child in properties.values_mut() {
            normalize_integer_minimum_equivalents_schema(child);
        }
    }

    if let Some(pattern_properties) = schema_object
        .get_mut("patternProperties")
        .and_then(Value::as_object_mut)
    {
        for child in pattern_properties.values_mut() {
            normalize_integer_minimum_equivalents_schema(child);
        }
    }

    if let Some(definitions) = schema_object
        .get_mut("$defs")
        .and_then(Value::as_object_mut)
    {
        for child in definitions.values_mut() {
            normalize_integer_minimum_equivalents_schema(child);
        }
    }

    if let Some(definitions) = schema_object
        .get_mut("definitions")
        .and_then(Value::as_object_mut)
    {
        for child in definitions.values_mut() {
            normalize_integer_minimum_equivalents_schema(child);
        }
    }

    normalize_integer_minimum_equivalents_child(schema_object.get_mut("items"));
    normalize_integer_minimum_equivalents_child(schema_object.get_mut("contains"));
    normalize_integer_minimum_equivalents_child(schema_object.get_mut("not"));
    normalize_integer_minimum_equivalents_child(schema_object.get_mut("if"));
    normalize_integer_minimum_equivalents_child(schema_object.get_mut("then"));
    normalize_integer_minimum_equivalents_child(schema_object.get_mut("else"));
    normalize_integer_minimum_equivalents_child(schema_object.get_mut("additionalProperties"));
    normalize_integer_minimum_equivalents_child(schema_object.get_mut("propertyNames"));
    normalize_integer_minimum_equivalents_child(schema_object.get_mut("unevaluatedItems"));
    normalize_integer_minimum_equivalents_child(schema_object.get_mut("unevaluatedProperties"));

    if let Some(children) = schema_object
        .get_mut("prefixItems")
        .and_then(Value::as_array_mut)
    {
        for child in children {
            normalize_integer_minimum_equivalents_schema(child);
        }
    }

    for keyword in ["allOf", "anyOf", "oneOf"] {
        if let Some(children) = schema_object.get_mut(keyword).and_then(Value::as_array_mut) {
            for child in children {
                normalize_integer_minimum_equivalents_schema(child);
            }
        }
    }

    if let Some(children) = schema_object
        .get_mut("dependentSchemas")
        .and_then(Value::as_object_mut)
    {
        for child in children.values_mut() {
            normalize_integer_minimum_equivalents_schema(child);
        }
    }
}

fn normalize_integer_minimum_equivalents_child(child: Option<&mut Value>) {
    let Some(child) = child else {
        return;
    };

    match child {
        Value::Object(_) | Value::Bool(true) => normalize_integer_minimum_equivalents_schema(child),
        Value::Array(children) => {
            for child in children {
                normalize_integer_minimum_equivalents_schema(child);
            }
        }
        _ => {}
    }
}

fn normalize_simple_nullable_property_schema(schema: &mut Value) {
    let Some(schema_object) = schema.as_object() else {
        return;
    };

    if let Some(normalized_type) = normalized_nullable_type_array(schema_object) {
        let mut normalized = schema_object.clone();
        normalized.insert("type".to_owned(), Value::String(normalized_type));
        *schema = Value::Object(normalized);
        return;
    }

    let Some(any_of) = schema_object.get("anyOf").and_then(Value::as_array) else {
        return;
    };
    if any_of.len() != 2 {
        return;
    }

    let non_null_branch = if is_null_schema(&any_of[0]) && !is_null_schema(&any_of[1]) {
        &any_of[1]
    } else if is_null_schema(&any_of[1]) && !is_null_schema(&any_of[0]) {
        &any_of[0]
    } else {
        return;
    };

    let wrapper_only_any_of = schema_object.len() == 1;
    if wrapper_only_any_of {
        *schema = non_null_branch.clone();
        return;
    }

    let Some(non_null_object) = non_null_branch.as_object() else {
        return;
    };

    let mut merged = non_null_object.clone();
    for (key, value) in schema_object {
        if key != "anyOf" {
            merged.insert(key.clone(), value.clone());
        }
    }
    *schema = Value::Object(merged);
}

fn normalized_nullable_type_array(schema_object: &Map<String, Value>) -> Option<String> {
    let schema_types = schema_object.get("type")?.as_array()?;
    if schema_types.len() != 2 {
        return None;
    }

    let mut non_null_type = None;
    let mut null_seen = false;
    for schema_type in schema_types {
        let schema_type = schema_type.as_str()?;
        if schema_type == "null" {
            null_seen = true;
        } else if non_null_type.is_none() {
            non_null_type = Some(schema_type.to_owned());
        } else {
            return None;
        }
    }

    null_seen.then_some(non_null_type?).filter(|schema_type| {
        matches!(
            schema_type.as_str(),
            "string" | "boolean" | "number" | "integer" | "object" | "array"
        )
    })
}

fn is_null_schema(schema: &Value) -> bool {
    schema
        .as_object()
        .and_then(|schema| schema.get("type"))
        .and_then(Value::as_str)
        .is_some_and(|schema_type| schema_type == "null")
}

fn normalize_integer_minimum_equivalent(schema_object: &mut Map<String, Value>) {
    if schema_object.contains_key("exclusiveMinimum")
        || !schema_type_is_integer(schema_object.get("type"))
    {
        return;
    }

    let Some(minimum) = schema_object.get("minimum") else {
        return;
    };
    let Some(exclusive_minimum) = normalized_integer_exclusive_minimum(minimum) else {
        return;
    };

    schema_object.remove("minimum");
    schema_object.insert("exclusiveMinimum".to_owned(), exclusive_minimum);
}

fn schema_type_is_integer(schema_type: Option<&Value>) -> bool {
    match schema_type {
        Some(Value::String(kind)) => kind == "integer",
        Some(Value::Array(kinds)) => kinds.iter().any(|kind| kind.as_str() == Some("integer")),
        _ => false,
    }
}

fn normalized_integer_exclusive_minimum(minimum: &Value) -> Option<Value> {
    if let Some(value) = minimum.as_i64() {
        return value
            .checked_sub(1)
            .map(|value| Value::Number(value.into()));
    }

    let value = minimum.as_u64()?;
    if value == 0 {
        Some(Value::Number((-1i64).into()))
    } else {
        Some(Value::Number((value - 1).into()))
    }
}

fn expand_schema_node(
    schema: &mut Value,
    source_schemas: &Map<String, Value>,
    stack: &mut Vec<String>,
) {
    let Some(reference_name) = schema
        .as_object()
        .and_then(|schema| schema.get("$ref"))
        .and_then(Value::as_str)
        .and_then(|reference| reference.strip_prefix(COMPONENT_REF_PREFIX))
        .map(str::to_owned)
    else {
        expand_schema_node_children(schema, source_schemas, stack);
        return;
    };

    if stack.iter().any(|name| name == &reference_name) {
        return;
    }

    let Some(referenced_schema) = source_schemas.get(&reference_name) else {
        return;
    };

    let mut expanded = referenced_schema.clone();
    stack.push(reference_name);
    expand_schema_node(&mut expanded, source_schemas, stack);
    stack.pop();
    *schema = expanded;
}

fn expand_selected_schema_node(
    schema: &mut Value,
    source_schemas: &Map<String, Value>,
    selected: &BTreeSet<&str>,
    stack: &mut Vec<String>,
) {
    let Some(reference_name) = schema
        .as_object()
        .and_then(|schema| schema.get("$ref"))
        .and_then(Value::as_str)
        .and_then(|reference| reference.strip_prefix(COMPONENT_REF_PREFIX))
        .map(str::to_owned)
    else {
        expand_selected_schema_node_children(schema, source_schemas, selected, stack);
        return;
    };

    if !selected.contains(reference_name.as_str()) {
        expand_selected_schema_node_children(schema, source_schemas, selected, stack);
        return;
    }

    if stack.iter().any(|name| name == &reference_name) {
        return;
    }

    let Some(referenced_schema) = source_schemas.get(&reference_name) else {
        return;
    };

    let mut expanded = referenced_schema.clone();
    stack.push(reference_name);
    expand_selected_schema_node(&mut expanded, source_schemas, selected, stack);
    stack.pop();
    *schema = expanded;
}

fn expand_schema_node_children(
    schema: &mut Value,
    source_schemas: &Map<String, Value>,
    stack: &mut Vec<String>,
) {
    let Some(schema_object) = schema.as_object_mut() else {
        return;
    };

    if let Some(properties) = schema_object
        .get_mut("properties")
        .and_then(Value::as_object_mut)
    {
        for child in properties.values_mut() {
            expand_schema_node(child, source_schemas, stack);
        }
    }

    if let Some(pattern_properties) = schema_object
        .get_mut("patternProperties")
        .and_then(Value::as_object_mut)
    {
        for child in pattern_properties.values_mut() {
            expand_schema_node(child, source_schemas, stack);
        }
    }

    if let Some(definitions) = schema_object
        .get_mut("$defs")
        .and_then(Value::as_object_mut)
    {
        for child in definitions.values_mut() {
            expand_schema_node(child, source_schemas, stack);
        }
    }

    if let Some(definitions) = schema_object
        .get_mut("definitions")
        .and_then(Value::as_object_mut)
    {
        for child in definitions.values_mut() {
            expand_schema_node(child, source_schemas, stack);
        }
    }

    expand_optional_schema_child(schema_object.get_mut("items"), source_schemas, stack);
    expand_optional_schema_child(schema_object.get_mut("contains"), source_schemas, stack);
    expand_optional_schema_child(schema_object.get_mut("not"), source_schemas, stack);
    expand_optional_schema_child(schema_object.get_mut("if"), source_schemas, stack);
    expand_optional_schema_child(schema_object.get_mut("then"), source_schemas, stack);
    expand_optional_schema_child(schema_object.get_mut("else"), source_schemas, stack);
    expand_optional_schema_child(
        schema_object.get_mut("additionalProperties"),
        source_schemas,
        stack,
    );
    expand_optional_schema_child(
        schema_object.get_mut("propertyNames"),
        source_schemas,
        stack,
    );
    expand_optional_schema_child(
        schema_object.get_mut("unevaluatedItems"),
        source_schemas,
        stack,
    );
    expand_optional_schema_child(
        schema_object.get_mut("unevaluatedProperties"),
        source_schemas,
        stack,
    );

    if let Some(children) = schema_object
        .get_mut("prefixItems")
        .and_then(Value::as_array_mut)
    {
        for child in children {
            expand_schema_node(child, source_schemas, stack);
        }
    }

    for keyword in ["allOf", "anyOf", "oneOf"] {
        if let Some(children) = schema_object.get_mut(keyword).and_then(Value::as_array_mut) {
            for child in children {
                expand_schema_node(child, source_schemas, stack);
            }
        }
    }

    if let Some(children) = schema_object
        .get_mut("dependentSchemas")
        .and_then(Value::as_object_mut)
    {
        for child in children.values_mut() {
            expand_schema_node(child, source_schemas, stack);
        }
    }
}

fn expand_selected_schema_node_children(
    schema: &mut Value,
    source_schemas: &Map<String, Value>,
    selected: &BTreeSet<&str>,
    stack: &mut Vec<String>,
) {
    let Some(schema_object) = schema.as_object_mut() else {
        return;
    };

    if let Some(properties) = schema_object
        .get_mut("properties")
        .and_then(Value::as_object_mut)
    {
        for child in properties.values_mut() {
            expand_selected_schema_node(child, source_schemas, selected, stack);
        }
    }

    if let Some(pattern_properties) = schema_object
        .get_mut("patternProperties")
        .and_then(Value::as_object_mut)
    {
        for child in pattern_properties.values_mut() {
            expand_selected_schema_node(child, source_schemas, selected, stack);
        }
    }

    if let Some(definitions) = schema_object
        .get_mut("$defs")
        .and_then(Value::as_object_mut)
    {
        for child in definitions.values_mut() {
            expand_selected_schema_node(child, source_schemas, selected, stack);
        }
    }

    if let Some(definitions) = schema_object
        .get_mut("definitions")
        .and_then(Value::as_object_mut)
    {
        for child in definitions.values_mut() {
            expand_selected_schema_node(child, source_schemas, selected, stack);
        }
    }

    expand_selected_optional_schema_child(
        schema_object.get_mut("items"),
        source_schemas,
        selected,
        stack,
    );
    expand_selected_optional_schema_child(
        schema_object.get_mut("contains"),
        source_schemas,
        selected,
        stack,
    );
    expand_selected_optional_schema_child(
        schema_object.get_mut("not"),
        source_schemas,
        selected,
        stack,
    );
    expand_selected_optional_schema_child(
        schema_object.get_mut("if"),
        source_schemas,
        selected,
        stack,
    );
    expand_selected_optional_schema_child(
        schema_object.get_mut("then"),
        source_schemas,
        selected,
        stack,
    );
    expand_selected_optional_schema_child(
        schema_object.get_mut("else"),
        source_schemas,
        selected,
        stack,
    );
    expand_selected_optional_schema_child(
        schema_object.get_mut("additionalProperties"),
        source_schemas,
        selected,
        stack,
    );
    expand_selected_optional_schema_child(
        schema_object.get_mut("propertyNames"),
        source_schemas,
        selected,
        stack,
    );
    expand_selected_optional_schema_child(
        schema_object.get_mut("unevaluatedItems"),
        source_schemas,
        selected,
        stack,
    );
    expand_selected_optional_schema_child(
        schema_object.get_mut("unevaluatedProperties"),
        source_schemas,
        selected,
        stack,
    );

    for keyword in ["allOf", "anyOf", "oneOf", "prefixItems"] {
        let Some(children) = schema_object.get_mut(keyword).and_then(Value::as_array_mut) else {
            continue;
        };
        for child in children {
            expand_selected_schema_node(child, source_schemas, selected, stack);
        }
    }
}

fn expand_selected_optional_schema_child(
    child: Option<&mut Value>,
    source_schemas: &Map<String, Value>,
    selected: &BTreeSet<&str>,
    stack: &mut Vec<String>,
) {
    if let Some(child) = child {
        expand_selected_schema_node(child, source_schemas, selected, stack);
    }
}

fn expand_optional_schema_child(
    child: Option<&mut Value>,
    source_schemas: &Map<String, Value>,
    stack: &mut Vec<String>,
) {
    let Some(child) = child else {
        return;
    };

    match child {
        Value::Object(_) => expand_schema_node(child, source_schemas, stack),
        Value::Array(children) => {
            for child in children {
                expand_schema_node(child, source_schemas, stack);
            }
        }
        _ => {}
    }
}

fn collect_schema_refs(value: &Value, pending_refs: &mut VecDeque<String>) {
    match value {
        Value::Object(map) => {
            if let Some(reference) = map.get("$ref").and_then(Value::as_str) {
                pending_refs.push_back(reference.to_owned());
            }
            for child in map.values() {
                collect_schema_refs(child, pending_refs);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_schema_refs(item, pending_refs);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use serde_json::{Value, json};

    use super::{
        SchemaRootTargets, exclude_numeric_schema_formats, exclude_parameter_descriptions,
        exclude_parameter_fields, exclude_request_body_fields, exclude_response_codes,
        exclude_response_schema_defaults, exclude_schema_metadata_fields, exclude_schemas,
        exclude_unreferenced_schemas, expand_schema_refs, flatten_single_entry_all_of,
        for_each_parameter_mut, for_each_request_body_mut, for_each_response_mut,
        for_each_schema_root_mut, normalize_empty_components_schemas,
        normalize_integer_minimum_equivalents, normalize_integral_schema_defaults,
        normalize_optional_parameter_nullability, normalize_optional_property_nullability,
        normalize_single_value_enums_to_const, normalize_true_schemas_to_empty_objects,
        sort_parameters, subset_for_operations, subset_for_paths,
    };

    #[test]
    fn subset_for_paths_keeps_requested_paths_and_transitive_schema_closure() {
        let source = json!({
            "openapi": "3.1.1",
            "info": {
                "title": "example",
                "version": "0.1.0"
            },
            "paths": {
                "/keep": {
                    "get": {
                        "responses": {
                            "200": {
                                "content": {
                                    "application/json": {
                                        "schema": {
                                            "$ref": "#/components/schemas/A"
                                        }
                                    }
                                }
                            }
                        }
                    }
                },
                "/drop": {
                    "get": {
                        "responses": {
                            "200": {
                                "content": {
                                    "application/json": {
                                        "schema": {
                                            "$ref": "#/components/schemas/C"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            },
            "components": {
                "schemas": {
                    "A": {
                        "type": "object",
                        "properties": {
                            "b": { "$ref": "#/components/schemas/B" }
                        }
                    },
                    "B": {
                        "type": "string"
                    },
                    "C": {
                        "type": "number"
                    }
                }
            }
        });

        let subset = subset_for_paths(&source, &["/keep"]);

        assert_eq!(
            subset["paths"]
                .as_object()
                .expect("subset paths should be present")
                .keys()
                .cloned()
                .collect::<BTreeSet<_>>(),
            BTreeSet::from(["/keep".to_owned()])
        );

        assert_eq!(
            subset["components"]["schemas"]
                .as_object()
                .expect("subset schemas should be present")
                .keys()
                .cloned()
                .collect::<BTreeSet<_>>(),
            BTreeSet::from(["A".to_owned(), "B".to_owned()])
        );
    }

    #[test]
    fn subset_for_operations_keeps_only_requested_methods() {
        let source = json!({
            "openapi": "3.1.1",
            "info": {
                "title": "example",
                "version": "0.1.0"
            },
            "paths": {
                "/config": {
                    "get": {
                        "responses": {
                            "200": {
                                "content": {
                                    "application/json": {
                                        "schema": {
                                            "$ref": "#/components/schemas/A"
                                        }
                                    }
                                }
                            }
                        }
                    },
                    "patch": {
                        "responses": {
                            "200": {
                                "content": {
                                    "application/json": {
                                        "schema": {
                                            "$ref": "#/components/schemas/B"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            },
            "components": {
                "schemas": {
                    "A": { "type": "string" },
                    "B": { "type": "number" }
                }
            }
        });

        let subset = subset_for_operations(&source, &[("/config", "patch")]);

        assert_eq!(
            subset["paths"],
            json!({
                "/config": {
                    "patch": {
                        "responses": {
                            "200": {
                                "content": {
                                    "application/json": {
                                        "schema": {
                                            "$ref": "#/components/schemas/B"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            })
        );
        assert_eq!(
            subset["components"]["schemas"],
            json!({
                "B": { "type": "number" }
            })
        );
    }

    #[test]
    fn subset_for_paths_ignores_non_schema_refs() {
        let source = json!({
            "openapi": "3.1.1",
            "info": {
                "title": "example",
                "version": "0.1.0"
            },
            "paths": {
                "/keep": {
                    "get": {
                        "responses": {
                            "200": {
                                "$ref": "#/components/responses/SharedResponse"
                            }
                        }
                    }
                }
            },
            "components": {
                "schemas": {
                    "Unused": {
                        "type": "string"
                    }
                }
            }
        });

        let subset = subset_for_paths(&source, &["/keep"]);

        assert_eq!(
            subset["paths"]["/keep"]["get"]["responses"]["200"]["$ref"],
            Value::String("#/components/responses/SharedResponse".to_owned())
        );
        assert!(
            subset["components"]["schemas"]
                .as_object()
                .expect("subset schemas should be present")
                .is_empty()
        );
    }

    #[test]
    fn exclude_schemas_removes_only_named_components() {
        let source = json!({
            "components": {
                "schemas": {
                    "A": { "type": "string" },
                    "B": { "type": "number" }
                }
            }
        });

        let excluded = exclude_schemas(&source, &["A"]);

        assert_eq!(
            excluded["components"]["schemas"],
            json!({
                "B": { "type": "number" }
            })
        );
    }

    #[test]
    fn exclude_unreferenced_schemas_keeps_only_reachable_closure() {
        let source = json!({
            "openapi": "3.1.1",
            "info": {
                "title": "example",
                "version": "0.1.0"
            },
            "paths": {
                "/keep": {
                    "get": {
                        "responses": {
                            "200": {
                                "content": {
                                    "application/json": {
                                        "schema": {
                                            "$ref": "#/components/schemas/A"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            },
            "components": {
                "schemas": {
                    "A": {
                        "type": "object",
                        "properties": {
                            "b": { "$ref": "#/components/schemas/B" }
                        }
                    },
                    "B": {
                        "type": "string"
                    },
                    "Unused": {
                        "type": "number"
                    }
                }
            }
        });

        let pruned = exclude_unreferenced_schemas(&source);

        assert_eq!(
            pruned["components"]["schemas"],
            json!({
                "A": {
                    "type": "object",
                    "properties": {
                        "b": { "$ref": "#/components/schemas/B" }
                    }
                },
                "B": {
                    "type": "string"
                }
            })
        );
    }

    #[test]
    fn exclude_parameter_fields_removes_style_and_schema_defaults_only() {
        let source = json!({
            "paths": {
                "/keep": {
                    "get": {
                        "parameters": [
                            {
                                "in": "query",
                                "name": "directory",
                                "style": "form",
                                "schema": {
                                    "default": null,
                                    "type": "string"
                                }
                            }
                        ],
                        "responses": {
                            "200": {
                                "description": "ok"
                            }
                        }
                    }
                }
            }
        });

        let normalized = exclude_parameter_fields(&source, &["style", "schema/default"]);

        assert_eq!(
            normalized,
            json!({
                "paths": {
                    "/keep": {
                        "get": {
                            "parameters": [
                                {
                                    "in": "query",
                                    "name": "directory",
                                    "schema": {
                                        "type": "string"
                                    }
                                }
                            ],
                            "responses": {
                                "200": {
                                    "description": "ok"
                                }
                            }
                        }
                    }
                }
            })
        );
    }

    #[test]
    fn sort_parameters_orders_operation_parameters_stably() {
        let source = json!({
            "paths": {
                "/keep": {
                    "get": {
                        "parameters": [
                            { "in": "query", "name": "workspace" },
                            { "in": "path", "name": "requestID" },
                            { "in": "query", "name": "directory" }
                        ]
                    }
                }
            }
        });

        let normalized = sort_parameters(&source);

        assert_eq!(
            normalized["paths"]["/keep"]["get"]["parameters"],
            json!([
                { "in": "path", "name": "requestID" },
                { "in": "query", "name": "directory" },
                { "in": "query", "name": "workspace" }
            ])
        );
    }

    #[test]
    fn exclude_request_body_fields_removes_relative_fields_only() {
        let source = json!({
            "paths": {
                "/keep": {
                    "post": {
                        "requestBody": {
                            "required": true,
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "object"
                                    }
                                }
                            }
                        },
                        "responses": {
                            "200": {
                                "description": "ok"
                            }
                        }
                    }
                }
            }
        });

        let normalized = exclude_request_body_fields(&source, &["required"]);

        assert_eq!(
            normalized,
            json!({
                "paths": {
                    "/keep": {
                        "post": {
                            "requestBody": {
                                "content": {
                                    "application/json": {
                                        "schema": {
                                            "type": "object"
                                        }
                                    }
                                }
                            },
                            "responses": {
                                "200": {
                                    "description": "ok"
                                }
                            }
                        }
                    }
                }
            })
        );
    }

    #[test]
    fn for_each_parameter_mut_touches_only_parameter_objects() {
        let mut source = json!({
            "paths": {
                "/keep": {
                    "get": {
                        "parameters": [
                            {
                                "in": "query",
                                "name": "directory",
                                "schema": { "type": "string" }
                            }
                        ],
                        "requestBody": {
                            "content": {
                                "application/json": {
                                    "schema": { "type": "object" }
                                }
                            }
                        },
                        "responses": {
                            "200": {
                                "content": {
                                    "application/json": {
                                        "schema": { "type": "number" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });

        for_each_parameter_mut(&mut source, |parameter| {
            parameter.insert("x-visited".to_owned(), json!(true));
        });

        assert_eq!(
            source["paths"]["/keep"]["get"]["parameters"][0]["x-visited"],
            json!(true)
        );
        assert!(
            source["paths"]["/keep"]["get"]["requestBody"]
                .as_object()
                .is_some_and(|request_body| !request_body.contains_key("x-visited"))
        );
        assert!(
            source["paths"]["/keep"]["get"]["responses"]["200"]
                .as_object()
                .is_some_and(|response| !response.contains_key("x-visited"))
        );
    }

    #[test]
    fn for_each_request_body_mut_touches_only_request_bodies() {
        let mut source = json!({
            "paths": {
                "/keep": {
                    "post": {
                        "parameters": [
                            {
                                "in": "query",
                                "name": "directory",
                                "schema": { "type": "string" }
                            }
                        ],
                        "requestBody": {
                            "content": {
                                "application/json": {
                                    "schema": { "type": "object" }
                                }
                            }
                        },
                        "responses": {
                            "200": {
                                "description": "ok"
                            }
                        }
                    }
                }
            }
        });

        for_each_request_body_mut(&mut source, |request_body| {
            request_body.insert("x-visited".to_owned(), json!(true));
        });

        assert_eq!(
            source["paths"]["/keep"]["post"]["requestBody"]["x-visited"],
            json!(true)
        );
        assert!(
            source["paths"]["/keep"]["post"]["parameters"][0]
                .as_object()
                .is_some_and(|parameter| !parameter.contains_key("x-visited"))
        );
        assert!(
            source["paths"]["/keep"]["post"]["responses"]["200"]
                .as_object()
                .is_some_and(|response| !response.contains_key("x-visited"))
        );
    }

    #[test]
    fn for_each_response_mut_touches_only_responses() {
        let mut source = json!({
            "paths": {
                "/keep": {
                    "get": {
                        "parameters": [
                            {
                                "in": "query",
                                "name": "directory",
                                "schema": { "type": "string" }
                            }
                        ],
                        "requestBody": {
                            "content": {
                                "application/json": {
                                    "schema": { "type": "object" }
                                }
                            }
                        },
                        "responses": {
                            "200": {
                                "description": "ok"
                            }
                        }
                    }
                }
            }
        });

        for_each_response_mut(&mut source, |response| {
            response.insert("x-visited".to_owned(), json!(true));
        });

        assert_eq!(
            source["paths"]["/keep"]["get"]["responses"]["200"]["x-visited"],
            json!(true)
        );
        assert!(
            source["paths"]["/keep"]["get"]["parameters"][0]
                .as_object()
                .is_some_and(|parameter| !parameter.contains_key("x-visited"))
        );
        assert!(
            source["paths"]["/keep"]["get"]["requestBody"]
                .as_object()
                .is_some_and(|request_body| !request_body.contains_key("x-visited"))
        );
    }

    #[test]
    fn for_each_schema_root_mut_all_targets_all_schema_locations() {
        let mut source = json!({
            "components": {
                "schemas": {
                    "Example": { "type": "string" }
                }
            },
            "paths": {
                "/keep": {
                    "post": {
                        "parameters": [
                            {
                                "in": "query",
                                "name": "directory",
                                "schema": { "type": "string" }
                            }
                        ],
                        "requestBody": {
                            "content": {
                                "application/json": {
                                    "schema": { "type": "object" }
                                }
                            }
                        },
                        "responses": {
                            "200": {
                                "content": {
                                    "application/json": {
                                        "schema": { "type": "number" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });

        for_each_schema_root_mut(&mut source, SchemaRootTargets::ALL, |schema| {
            schema
                .as_object_mut()
                .expect("schema root should be an object")
                .insert("x-schema-root".to_owned(), json!(true));
        });

        assert_eq!(
            source["components"]["schemas"]["Example"]["x-schema-root"],
            json!(true)
        );
        assert_eq!(
            source["paths"]["/keep"]["post"]["parameters"][0]["schema"]["x-schema-root"],
            json!(true)
        );
        assert_eq!(
            source["paths"]["/keep"]["post"]["requestBody"]["content"]["application/json"]["schema"]
                ["x-schema-root"],
            json!(true)
        );
        assert_eq!(
            source["paths"]["/keep"]["post"]["responses"]["200"]["content"]["application/json"]["schema"]
                ["x-schema-root"],
            json!(true)
        );
    }

    #[test]
    fn for_each_schema_root_mut_response_only_touches_only_response_schemas() {
        let mut source = json!({
            "components": {
                "schemas": {
                    "Example": { "type": "string" }
                }
            },
            "paths": {
                "/keep": {
                    "post": {
                        "parameters": [
                            {
                                "in": "query",
                                "name": "directory",
                                "schema": { "type": "string" }
                            }
                        ],
                        "requestBody": {
                            "content": {
                                "application/json": {
                                    "schema": { "type": "object" }
                                }
                            }
                        },
                        "responses": {
                            "200": {
                                "content": {
                                    "application/json": {
                                        "schema": { "type": "number" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });

        for_each_schema_root_mut(&mut source, SchemaRootTargets::RESPONSES_ONLY, |schema| {
            schema
                .as_object_mut()
                .expect("response schema root should be an object")
                .insert("x-schema-root".to_owned(), json!(true));
        });

        assert!(
            source["components"]["schemas"]["Example"]
                .as_object()
                .is_some_and(|schema| !schema.contains_key("x-schema-root"))
        );
        assert!(
            source["paths"]["/keep"]["post"]["parameters"][0]["schema"]
                .as_object()
                .is_some_and(|schema| !schema.contains_key("x-schema-root"))
        );
        assert!(
            source["paths"]["/keep"]["post"]["requestBody"]["content"]["application/json"]["schema"]
                .as_object()
                .is_some_and(|schema| !schema.contains_key("x-schema-root"))
        );
        assert_eq!(
            source["paths"]["/keep"]["post"]["responses"]["200"]["content"]["application/json"]["schema"]
                ["x-schema-root"],
            json!(true)
        );
    }

    #[test]
    fn normalize_single_value_enums_to_const_rewrites_schema_nodes() {
        let source = json!({
            "components": {
                "schemas": {
                    "Example": {
                        "type": "object",
                        "properties": {
                            "name": {
                                "type": "string",
                                "enum": ["NotFoundError"]
                            }
                        }
                    }
                }
            },
            "paths": {
                "/keep": {
                    "post": {
                        "requestBody": {
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "object",
                                        "properties": {
                                            "reply": {
                                                "type": "string",
                                                "enum": ["once"]
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });

        let normalized = normalize_single_value_enums_to_const(&source);

        assert_eq!(
            normalized["components"]["schemas"]["Example"]["properties"]["name"],
            json!({
                "type": "string",
                "const": "NotFoundError"
            })
        );
        assert_eq!(
            normalized["paths"]["/keep"]["post"]["requestBody"]["content"]["application/json"]["schema"]
                ["properties"]["reply"],
            json!({
                "type": "string",
                "const": "once"
            })
        );
    }

    #[test]
    fn normalize_true_schemas_to_empty_objects_rewrites_schema_nodes_only() {
        let source = json!({
            "components": {
                "schemas": {
                    "Example": {
                        "type": "object",
                        "properties": {
                            "data": true,
                            "nested": {
                                "type": "object",
                                "properties": {
                                    "value": true
                                }
                            },
                            "blocked": false
                        }
                    }
                }
            },
            "paths": {
                "/keep": {
                    "post": {
                        "requestBody": {
                            "required": true,
                            "content": {
                                "application/json": {
                                    "schema": true
                                }
                            }
                        }
                    }
                }
            }
        });

        let normalized = normalize_true_schemas_to_empty_objects(&source);

        assert_eq!(
            normalized["components"]["schemas"]["Example"]["properties"]["data"],
            json!({})
        );
        assert_eq!(
            normalized["components"]["schemas"]["Example"]["properties"]["nested"]["properties"]["value"],
            json!({})
        );
        assert_eq!(
            normalized["paths"]["/keep"]["post"]["requestBody"]["content"]["application/json"]["schema"],
            json!({})
        );
        assert_eq!(
            normalized["paths"]["/keep"]["post"]["requestBody"]["required"],
            json!(true)
        );
        assert_eq!(
            normalized["components"]["schemas"]["Example"]["properties"]["blocked"],
            json!(false)
        );
    }

    #[test]
    fn normalize_optional_property_nullability_rewrites_non_required_properties_only() {
        let source = json!({
            "components": {
                "schemas": {
                    "Example": {
                        "type": "object",
                        "required": ["requiredValue"],
                        "properties": {
                            "optionalString": {
                                "type": ["string", "null"]
                            },
                            "optionalObject": {
                                "anyOf": [
                                    {
                                        "type": "object",
                                        "properties": {
                                            "value": { "type": ["string", "null"] }
                                        }
                                    },
                                    { "type": "null" }
                                ]
                            },
                            "requiredValue": {
                                "type": ["string", "null"]
                            },
                            "ambiguous": {
                                "anyOf": [
                                    { "type": "string" },
                                    { "type": "integer" },
                                    { "type": "null" }
                                ]
                            }
                        }
                    }
                }
            },
            "paths": {
                "/keep": {
                    "post": {
                        "requestBody": {
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "array",
                                        "items": {
                                            "type": ["string", "null"]
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });

        let normalized = normalize_optional_property_nullability(&source);

        assert_eq!(
            normalized["components"]["schemas"]["Example"]["properties"]["optionalString"],
            json!({ "type": "string" })
        );
        assert_eq!(
            normalized["components"]["schemas"]["Example"]["properties"]["optionalObject"],
            json!({
                "type": "object",
                "properties": {
                    "value": { "type": "string" }
                }
            })
        );
        assert_eq!(
            normalized["components"]["schemas"]["Example"]["properties"]["requiredValue"],
            json!({ "type": ["string", "null"] })
        );
        assert_eq!(
            normalized["components"]["schemas"]["Example"]["properties"]["ambiguous"],
            json!({
                "anyOf": [
                    { "type": "string" },
                    { "type": "integer" },
                    { "type": "null" }
                ]
            })
        );
        assert_eq!(
            normalized["paths"]["/keep"]["post"]["requestBody"]["content"]["application/json"]["schema"]
                ["items"],
            json!({ "type": ["string", "null"] })
        );
    }

    #[test]
    fn normalize_optional_property_nullability_preserves_wrapper_metadata() {
        let source = json!({
            "components": {
                "schemas": {
                    "Example": {
                        "type": "object",
                        "properties": {
                            "status": {
                                "description": "Current status",
                                "anyOf": [
                                    {
                                        "type": "string",
                                        "enum": ["pending", "done"]
                                    },
                                    { "type": "null" }
                                ]
                            }
                        }
                    }
                }
            }
        });

        let normalized = normalize_optional_property_nullability(&source);

        assert_eq!(
            normalized["components"]["schemas"]["Example"]["properties"]["status"],
            json!({
                "description": "Current status",
                "type": "string",
                "enum": ["pending", "done"]
            })
        );
    }

    #[test]
    fn normalize_optional_parameter_nullability_rewrites_optional_parameters_only() {
        let source = json!({
            "paths": {
                "/keep": {
                    "get": {
                        "parameters": [
                            {
                                "in": "query",
                                "name": "optionalFlag",
                                "schema": {
                                    "description": "Optional flag",
                                    "type": ["boolean", "null"]
                                }
                            },
                            {
                                "in": "query",
                                "name": "optionalWrapped",
                                "schema": {
                                    "description": "Wrapped value",
                                    "anyOf": [
                                        { "type": "string" },
                                        { "type": "null" }
                                    ]
                                }
                            },
                            {
                                "in": "query",
                                "name": "requiredFlag",
                                "required": true,
                                "schema": {
                                    "type": ["boolean", "null"]
                                }
                            }
                        ]
                    }
                }
            }
        });

        let normalized = normalize_optional_parameter_nullability(&source);

        assert_eq!(
            normalized["paths"]["/keep"]["get"]["parameters"][0]["schema"],
            json!({
                "description": "Optional flag",
                "type": "boolean"
            })
        );
        assert_eq!(
            normalized["paths"]["/keep"]["get"]["parameters"][1]["schema"],
            json!({
                "description": "Wrapped value",
                "type": "string"
            })
        );
        assert_eq!(
            normalized["paths"]["/keep"]["get"]["parameters"][2]["schema"],
            json!({
                "type": ["boolean", "null"]
            })
        );
    }

    #[test]
    fn exclude_numeric_schema_formats_removes_numeric_format_only() {
        let source = json!({
            "components": {
                "schemas": {
                    "Example": {
                        "type": "object",
                        "properties": {
                            "count": {
                                "type": "integer",
                                "format": "int64"
                            },
                            "ratio": {
                                "type": "number",
                                "format": "double"
                            },
                            "label": {
                                "type": "string",
                                "format": "uuid"
                            },
                            "maybeCount": {
                                "type": ["integer", "null"],
                                "format": "int64"
                            }
                        }
                    }
                }
            },
            "paths": {
                "/keep": {
                    "get": {
                        "requestBody": {
                            "required": true
                        }
                    }
                }
            }
        });

        let normalized = exclude_numeric_schema_formats(&source);

        assert_eq!(
            normalized["components"]["schemas"]["Example"]["properties"]["count"],
            json!({ "type": "integer" })
        );
        assert_eq!(
            normalized["components"]["schemas"]["Example"]["properties"]["ratio"],
            json!({ "type": "number" })
        );
        assert_eq!(
            normalized["components"]["schemas"]["Example"]["properties"]["label"],
            json!({ "type": "string", "format": "uuid" })
        );
        assert_eq!(
            normalized["components"]["schemas"]["Example"]["properties"]["maybeCount"],
            json!({ "type": ["integer", "null"] })
        );
        assert_eq!(
            normalized["paths"]["/keep"]["get"]["requestBody"]["required"],
            json!(true)
        );
    }

    #[test]
    fn normalize_integer_minimum_equivalents_rewrites_integer_minimum_only() {
        let source = json!({
            "components": {
                "schemas": {
                    "Example": {
                        "type": "object",
                        "properties": {
                            "steps": {
                                "type": "integer",
                                "minimum": 1,
                                "maximum": 10
                            },
                            "offset": {
                                "type": ["integer", "null"],
                                "minimum": 5
                            },
                            "ratio": {
                                "type": "number",
                                "minimum": 1
                            },
                            "alreadyExclusive": {
                                "type": "integer",
                                "exclusiveMinimum": 0
                            }
                        }
                    }
                }
            }
        });

        let normalized = normalize_integer_minimum_equivalents(&source);

        assert_eq!(
            normalized["components"]["schemas"]["Example"]["properties"]["steps"],
            json!({
                "type": "integer",
                "exclusiveMinimum": 0,
                "maximum": 10
            })
        );
        assert_eq!(
            normalized["components"]["schemas"]["Example"]["properties"]["offset"],
            json!({
                "type": ["integer", "null"],
                "exclusiveMinimum": 4
            })
        );
        assert_eq!(
            normalized["components"]["schemas"]["Example"]["properties"]["ratio"],
            json!({
                "type": "number",
                "minimum": 1
            })
        );
        assert_eq!(
            normalized["components"]["schemas"]["Example"]["properties"]["alreadyExclusive"],
            json!({
                "type": "integer",
                "exclusiveMinimum": 0
            })
        );
    }

    #[test]
    fn normalize_integral_schema_defaults_rewrites_only_integral_defaults() {
        let source = json!({
            "components": {
                "schemas": {
                    "Example": {
                        "type": "object",
                        "properties": {
                            "count": {
                                "type": "number",
                                "default": 5000.0
                            },
                            "ratio": {
                                "type": "number",
                                "default": 1.5
                            },
                            "label": {
                                "type": "string",
                                "default": "hello"
                            }
                        }
                    }
                }
            },
            "paths": {
                "/keep": {
                    "post": {
                        "requestBody": {
                            "required": true
                        }
                    }
                }
            }
        });

        let normalized = normalize_integral_schema_defaults(&source);

        assert_eq!(
            normalized["components"]["schemas"]["Example"]["properties"]["count"],
            json!({ "type": "number", "default": 5000 })
        );
        assert_eq!(
            normalized["components"]["schemas"]["Example"]["properties"]["ratio"],
            json!({ "type": "number", "default": 1.5 })
        );
        assert_eq!(
            normalized["components"]["schemas"]["Example"]["properties"]["label"],
            json!({ "type": "string", "default": "hello" })
        );
        assert_eq!(
            normalized["paths"]["/keep"]["post"]["requestBody"]["required"],
            json!(true)
        );
    }

    #[test]
    fn exclude_response_schema_defaults_removes_defaults_only_from_response_schemas() {
        let source = json!({
            "paths": {
                "/example": {
                    "get": {
                        "parameters": [
                            {
                                "in": "query",
                                "name": "auto",
                                "schema": {
                                    "type": "boolean",
                                    "default": false
                                }
                            }
                        ],
                        "requestBody": {
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "object",
                                        "properties": {
                                            "duration": {
                                                "type": "number",
                                                "default": 5000
                                            }
                                        }
                                    }
                                }
                            }
                        },
                        "responses": {
                            "200": {
                                "content": {
                                    "application/json": {
                                        "schema": {
                                            "type": "object",
                                            "default": {},
                                            "properties": {
                                                "commands": {
                                                    "type": "object",
                                                    "default": {},
                                                    "properties": {
                                                        "start": {
                                                            "type": "string"
                                                        }
                                                    }
                                                },
                                                "default": {
                                                    "type": "object",
                                                    "properties": {
                                                        "label": {
                                                            "type": "string",
                                                            "default": "x"
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            },
                            "default": {
                                "description": "unexpected error"
                            }
                        }
                    }
                }
            }
        });

        let normalized = exclude_response_schema_defaults(&source);

        assert_eq!(
            normalized["paths"]["/example"]["get"]["parameters"][0]["schema"],
            json!({
                "type": "boolean",
                "default": false
            })
        );
        assert_eq!(
            normalized["paths"]["/example"]["get"]["requestBody"]["content"]["application/json"]["schema"],
            json!({
                "type": "object",
                "properties": {
                    "duration": {
                        "type": "number",
                        "default": 5000
                    }
                }
            })
        );
        assert_eq!(
            normalized["paths"]["/example"]["get"]["responses"]["200"]["content"]["application/json"]
                ["schema"],
            json!({
                "type": "object",
                "properties": {
                    "commands": {
                        "type": "object",
                        "properties": {
                            "start": {
                                "type": "string"
                            }
                        }
                    },
                    "default": {
                        "type": "object",
                        "properties": {
                            "label": {
                                "type": "string"
                            }
                        }
                    }
                }
            })
        );
        assert_eq!(
            normalized["paths"]["/example"]["get"]["responses"]["default"],
            json!({
                "description": "unexpected error"
            })
        );
    }

    #[test]
    fn flatten_single_entry_all_of_merges_inner_schema_and_preserves_metadata() {
        let source = json!({
            "components": {
                "schemas": {
                    "Example": {
                        "type": "object",
                        "properties": {
                            "level": {
                                "allOf": [
                                    {
                                        "type": "string",
                                        "enum": ["debug", "info", "warn", "error"]
                                    }
                                ],
                                "description": "Log level"
                            },
                            "nested": {
                                "allOf": [
                                    {
                                        "type": "object",
                                        "properties": {
                                            "value": { "type": "string" }
                                        }
                                    }
                                ]
                            }
                        }
                    }
                }
            }
        });

        let normalized = flatten_single_entry_all_of(&source);

        assert_eq!(
            normalized["components"]["schemas"]["Example"]["properties"]["level"],
            json!({
                "description": "Log level",
                "type": "string",
                "enum": ["debug", "info", "warn", "error"]
            })
        );
        assert_eq!(
            normalized["components"]["schemas"]["Example"]["properties"]["nested"],
            json!({
                "type": "object",
                "properties": {
                    "value": { "type": "string" }
                }
            })
        );
    }

    #[test]
    fn normalize_empty_components_schemas_ensures_schemas_object_exists() {
        let source = json!({
            "components": {}
        });

        let normalized = normalize_empty_components_schemas(&source);

        assert_eq!(
            normalized["components"],
            json!({
                "schemas": {}
            })
        );
    }

    #[test]
    fn expand_schema_refs_inlines_component_refs_in_schema_positions() {
        let source = json!({
            "components": {
                "schemas": {
                    "QuestionAnswer": {
                        "type": "array",
                        "items": {
                            "type": "string"
                        }
                    }
                }
            },
            "paths": {
                "/keep": {
                    "post": {
                        "requestBody": {
                            "required": true,
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "object",
                                        "properties": {
                                            "answers": {
                                                "type": "array",
                                                "items": {
                                                    "$ref": "#/components/schemas/QuestionAnswer"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });

        let expanded = expand_schema_refs(&source);

        assert_eq!(
            expanded["paths"]["/keep"]["post"]["requestBody"]["content"]["application/json"]["schema"]
                ["properties"]["answers"]["items"],
            json!({
                "type": "array",
                "items": {
                    "type": "string"
                }
            })
        );
        assert_eq!(
            expanded["paths"]["/keep"]["post"]["requestBody"]["required"],
            json!(true)
        );
    }

    #[test]
    fn exclude_schema_metadata_fields_strip_fields_from_each_schema_node() {
        let source = json!({
            "components": {
                "schemas": {
                    "Example": {
                        "type": "object",
                        "additionalProperties": {},
                        "propertyNames": {
                            "type": "string"
                        },
                        "properties": {
                            "nested": {
                                "type": "object",
                                "additionalProperties": true
                            }
                        }
                    }
                }
            }
        });

        let normalized =
            exclude_schema_metadata_fields(&source, &["additionalProperties", "propertyNames"]);

        assert_eq!(
            normalized,
            json!({
                "components": {
                    "schemas": {
                        "Example": {
                            "type": "object",
                            "properties": {
                                "nested": {
                                    "type": "object"
                                }
                            }
                        }
                    }
                }
            })
        );
    }

    #[test]
    fn composed_compat_style_normalization_pipeline_preserves_expected_shape() {
        let source = json!({
            "openapi": "3.1.1",
            "info": {
                "title": "example",
                "version": "0.1.0"
            },
            "paths": {
                "/example": {
                    "get": {
                        "parameters": [
                            {
                                "in": "query",
                                "name": "workspace",
                                "description": "Workspace filter",
                                "style": "form",
                                "schema": {
                                    "default": null,
                                    "type": ["boolean", "null"]
                                }
                            },
                            {
                                "in": "path",
                                "name": "id",
                                "schema": {
                                    "type": "string"
                                }
                            }
                        ],
                        "requestBody": {
                            "required": true,
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "object",
                                        "properties": {
                                            "count": {
                                                "type": "number",
                                                "default": 5000.0
                                            },
                                            "mode": {
                                                "type": "string",
                                                "enum": ["auto"]
                                            }
                                        }
                                    }
                                }
                            }
                        },
                        "responses": {
                            "200": {
                                "content": {
                                    "application/json": {
                                        "schema": {
                                            "$ref": "#/components/schemas/Used"
                                        }
                                    }
                                }
                            },
                            "415": { "description": "wrong type" },
                            "422": { "description": "bad body" }
                        }
                    }
                }
            },
            "components": {
                "schemas": {
                    "Used": {
                        "type": "object",
                        "additionalProperties": {},
                        "properties": {
                            "status": {
                                "type": "string",
                                "enum": ["ok"]
                            },
                            "config": true,
                            "limit": {
                                "type": "integer",
                                "minimum": 1,
                                "format": "int64"
                            },
                            "optional": {
                                "type": ["string", "null"]
                            },
                            "wrapped": {
                                "allOf": [
                                    { "type": "string" }
                                ],
                                "description": "Wrapped"
                            },
                            "nested": {
                                "type": "object",
                                "propertyNames": {
                                    "type": "string"
                                }
                            }
                        }
                    },
                    "Unused": {
                        "type": "string"
                    }
                }
            }
        });

        let normalized = exclude_schema_metadata_fields(
            &normalize_empty_components_schemas(&normalize_true_schemas_to_empty_objects(
                &normalize_single_value_enums_to_const(&normalize_integral_schema_defaults(
                    &normalize_integer_minimum_equivalents(
                        &normalize_optional_property_nullability(&exclude_request_body_fields(
                            &exclude_parameter_fields(
                                &exclude_parameter_descriptions(
                                    &normalize_optional_parameter_nullability(
                                        &exclude_unreferenced_schemas(
                                            &exclude_numeric_schema_formats(
                                                &exclude_response_schema_defaults(
                                                    &flatten_single_entry_all_of(
                                                        &expand_schema_refs(&sort_parameters(
                                                            &exclude_response_codes(
                                                                &source,
                                                                &[415, 422],
                                                            ),
                                                        )),
                                                    ),
                                                ),
                                            ),
                                        ),
                                    ),
                                ),
                                &["style", "schema/default"],
                            ),
                            &["required"],
                        )),
                    ),
                )),
            )),
            &["additionalProperties", "propertyNames"],
        );

        assert_eq!(
            normalized,
            json!({
                "openapi": "3.1.1",
                "info": {
                    "title": "example",
                    "version": "0.1.0"
                },
                "paths": {
                    "/example": {
                        "get": {
                            "parameters": [
                                {
                                    "in": "path",
                                    "name": "id",
                                    "schema": {
                                        "type": "string"
                                    }
                                },
                                {
                                    "in": "query",
                                    "name": "workspace",
                                    "schema": {
                                        "type": "boolean"
                                    }
                                }
                            ],
                            "requestBody": {
                                "content": {
                                    "application/json": {
                                        "schema": {
                                            "type": "object",
                                            "properties": {
                                                "count": {
                                                    "type": "number",
                                                    "default": 5000
                                                },
                                                "mode": {
                                                    "type": "string",
                                                    "const": "auto"
                                                }
                                            }
                                        }
                                    }
                                }
                            },
                            "responses": {
                                "200": {
                                    "content": {
                                        "application/json": {
                                            "schema": {
                                                "type": "object",
                                                "properties": {
                                                    "status": {
                                                        "type": "string",
                                                        "const": "ok"
                                                    },
                                                    "config": {},
                                                    "limit": {
                                                        "type": "integer",
                                                        "exclusiveMinimum": 0
                                                    },
                                                    "optional": {
                                                        "type": "string"
                                                    },
                                                    "wrapped": {
                                                        "description": "Wrapped",
                                                        "type": "string"
                                                    },
                                                    "nested": {
                                                        "type": "object"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                },
                "components": {
                    "schemas": {}
                }
            })
        );
    }

    #[test]
    fn exclude_response_codes_removes_matching_codes_from_operations() {
        let source = json!({
            "openapi": "3.1.1",
            "info": {
                "title": "example",
                "version": "0.1.0"
            },
            "paths": {
                "/keep": {
                    "get": {
                        "responses": {
                            "200": { "description": "ok" },
                            "415": { "description": "wrong type" },
                            "422": { "description": "bad body" }
                        }
                    }
                }
            },
            "components": {
                "schemas": {
                    "Unused": { "type": "string" }
                }
            }
        });

        let filtered = exclude_response_codes(&source, &[415, 422]);

        assert_eq!(
            filtered["paths"]["/keep"]["get"]["responses"]
                .as_object()
                .expect("responses should still be present")
                .keys()
                .cloned()
                .collect::<BTreeSet<_>>(),
            BTreeSet::from(["200".to_owned()])
        );
        assert_eq!(filtered["components"], source["components"]);
    }

    #[test]
    fn exclude_response_codes_leaves_non_matching_codes_untouched() {
        let source = json!({
            "openapi": "3.1.1",
            "info": {
                "title": "example",
                "version": "0.1.0"
            },
            "paths": {
                "/keep": {
                    "post": {
                        "responses": {
                            "200": { "description": "ok" },
                            "404": { "description": "missing" }
                        }
                    }
                }
            }
        });

        let filtered = exclude_response_codes(&source, &[415, 422]);
        assert_eq!(filtered, source);
    }
}
