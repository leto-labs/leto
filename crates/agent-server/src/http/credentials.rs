use agent_store::{CredentialEntry, CredentialStoreKey};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};

use super::AppState;
use super::errors::store_error_response;
use crate::types::{CredentialHealthRecord, CredentialRecord, UpdateCredentialHealthRequest};

pub(super) async fn list_credentials(State(server): State<AppState>) -> Response {
    let core = server.core();
    match core.store().credentials().list().await {
        Ok(records) => Json(credential_records(records)).into_response(),
        Err(error) => store_error_response(error),
    }
}

pub(super) async fn list_credential_health(State(server): State<AppState>) -> Response {
    let core = server.core();
    match core.store().credentials().list().await {
        Ok(records) => Json(credential_health_records(records)).into_response(),
        Err(error) => store_error_response(error),
    }
}

pub(super) async fn list_provider_credentials(
    State(server): State<AppState>,
    Path(provider): Path<String>,
) -> Response {
    let core = server.core();
    match core
        .store()
        .credentials()
        .list_for_provider(&provider)
        .await
    {
        Ok(records) => Json(credential_records(records)).into_response(),
        Err(error) => store_error_response(error),
    }
}

pub(super) async fn get_credential(
    State(server): State<AppState>,
    Path((provider, id)): Path<(String, String)>,
) -> Response {
    let core = server.core();
    match core.store().credentials().get((provider, id)).await {
        Ok(entry) => Json(entry).into_response(),
        Err(error) => store_error_response(error),
    }
}

pub(super) async fn create_credential(
    State(server): State<AppState>,
    Path((provider, id)): Path<(String, String)>,
    Json(credential): Json<CredentialEntry>,
) -> Response {
    let core = server.core();
    match core
        .store()
        .credentials()
        .create((provider, id), credential)
        .await
    {
        Ok(entry) => (StatusCode::CREATED, Json(entry)).into_response(),
        Err(error) => store_error_response(error),
    }
}

pub(super) async fn update_credential(
    State(server): State<AppState>,
    Path((provider, id)): Path<(String, String)>,
    Json(credential): Json<CredentialEntry>,
) -> Response {
    let core = server.core();
    match core
        .store()
        .credentials()
        .update((provider, id), credential)
        .await
    {
        Ok(entry) => Json(entry).into_response(),
        Err(error) => store_error_response(error),
    }
}

pub(super) async fn update_credential_health(
    State(server): State<AppState>,
    Path((provider, id)): Path<(String, String)>,
    Json(body): Json<UpdateCredentialHealthRequest>,
) -> Response {
    let core = server.core();
    match core
        .store()
        .credentials()
        .update_health(&provider, &id, &body.health)
        .await
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => store_error_response(error),
    }
}

pub(super) async fn delete_credential(
    State(server): State<AppState>,
    Path((provider, id)): Path<(String, String)>,
) -> Response {
    let core = server.core();
    match core.store().credentials().delete((provider, id)).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => store_error_response(error),
    }
}

fn credential_records(
    records: Vec<(CredentialStoreKey, CredentialEntry)>,
) -> Vec<CredentialRecord> {
    records
        .into_iter()
        .map(
            |((provider_name, credential_id), credential)| CredentialRecord {
                provider_name,
                credential_id,
                credential,
            },
        )
        .collect()
}

fn credential_health_records(
    records: Vec<(CredentialStoreKey, CredentialEntry)>,
) -> Vec<CredentialHealthRecord> {
    records
        .into_iter()
        .map(
            |((provider_name, credential_id), credential)| CredentialHealthRecord {
                provider_name,
                credential_id,
                health: credential.health,
            },
        )
        .collect()
}
