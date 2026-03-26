use serde_json::Value;

use crate::utils::openapi::{
    OpenApiGenerationOptions, exclude_numeric_schema_formats, exclude_parameter_descriptions,
    exclude_parameter_fields, exclude_request_body_fields, exclude_response_codes,
    exclude_response_schema_defaults, exclude_schema_metadata_fields, exclude_unreferenced_schemas,
    expand_schema_refs, expand_selected_schema_refs, flatten_single_entry_all_of,
    normalize_empty_components_schemas, normalize_integer_minimum_equivalents,
    normalize_integral_schema_defaults, normalize_optional_parameter_nullability,
    normalize_optional_property_nullability, normalize_single_value_enums_to_const,
    normalize_true_schemas_to_empty_objects, sort_parameters,
};

const OPENAPI_OPTIONS: OpenApiGenerationOptions<'static> = OpenApiGenerationOptions {
    title: "opencode",
    description: "opencode api",
    version: "0.0.3",
    openapi_version_override: Some("3.1.1"),
};
const PARAMETER_FIELDS_TO_EXCLUDE: &[&str] = &["style", "schema/default"];
const REQUEST_BODY_FIELDS_TO_EXCLUDE: &[&str] = &["required"];
const SCHEMA_METADATA_FIELDS_TO_EXCLUDE: &[&str] = &["additionalProperties", "propertyNames"];

pub(crate) fn opencode_openapi_options() -> OpenApiGenerationOptions<'static> {
    OPENAPI_OPTIONS
}

pub(crate) fn pinned_opencode_openapi() -> Value {
    serde_json::from_str(include_str!("../../../../../openapi/opencode.json"))
        .expect("pinned OpenAPI contract should parse")
}

pub(crate) fn normalize_opencode_route_doc(doc: &Value) -> Value {
    let doc = exclude_response_codes(doc, &[415, 422]);
    let doc = sort_parameters(&doc);
    let doc = expand_schema_refs(&doc);
    let doc = flatten_single_entry_all_of(&doc);
    let doc = exclude_response_schema_defaults(&doc);
    let doc = exclude_numeric_schema_formats(&doc);
    let doc = exclude_unreferenced_schemas(&doc);
    let doc = normalize_optional_parameter_nullability(&doc);
    let doc = exclude_parameter_descriptions(&doc);
    let doc = exclude_parameter_fields(&doc, PARAMETER_FIELDS_TO_EXCLUDE);
    let doc = exclude_request_body_fields(&doc, REQUEST_BODY_FIELDS_TO_EXCLUDE);
    let doc = normalize_optional_property_nullability(&doc);
    let doc = normalize_integer_minimum_equivalents(&doc);
    let doc = normalize_integral_schema_defaults(&doc);
    let doc = normalize_single_value_enums_to_const(&doc);
    let doc = normalize_true_schemas_to_empty_objects(&doc);
    let doc = normalize_empty_components_schemas(&doc);
    exclude_schema_metadata_fields(&doc, SCHEMA_METADATA_FIELDS_TO_EXCLUDE)
}

pub(crate) fn normalize_generated_opencode_route_doc(doc: &Value) -> Value {
    let doc = expand_selected_schema_refs(
        doc,
        &[
            "McpAddRequest",
            "McpAuthCallbackRequest",
            "ProviderOAuthAuthorizeRequest",
            "ProviderOAuthCallbackRequest",
        ],
    );
    normalize_opencode_route_doc(&doc)
}
