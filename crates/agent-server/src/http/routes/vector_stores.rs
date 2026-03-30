use axum::Json;
use axum::response::{IntoResponse, Response};
use chrono::Utc;
use provider_openai::{VectorStoreCreateRequest, VectorStoreFileCounts, VectorStoreObject};
use ulid::Ulid;

pub(in crate::http) async fn create_vector_store(
    Json(body): Json<VectorStoreCreateRequest>,
) -> Response {
    Json(VectorStoreObject {
        id: format!("vs_{}", Ulid::new().to_string().to_lowercase()),
        object: "vector_store".to_owned(),
        created_at: Some(Utc::now().timestamp()),
        name: normalize_optional_field(body.name),
        description: normalize_optional_field(body.description),
        bytes: Some(0),
        file_counts: Some(VectorStoreFileCounts {
            in_progress: 0,
            completed: 0,
            failed: 0,
            cancelled: 0,
            total: 0,
        }),
    })
    .into_response()
}

fn normalize_optional_field(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_owned())
    })
}

#[cfg(test)]
mod tests {
    use super::normalize_optional_field;

    #[test]
    fn normalize_optional_field_trims_and_drops_blank_values() {
        assert_eq!(
            normalize_optional_field(Some("  support faq  ".into())),
            Some("support faq".into())
        );
        assert_eq!(normalize_optional_field(Some("   ".into())), None);
        assert_eq!(normalize_optional_field(None), None);
    }
}
