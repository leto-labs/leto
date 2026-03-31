#[derive(Debug, Clone, Copy)]
pub(super) enum CompatRequestError {
    InvalidRequest(&'static str),
    MissingBearerToken,
}

impl IntoResponse for CompatRequestError {
    fn into_response(self) -> Response {
        match self {
            Self::InvalidRequest(code) => invalid_request(code),
            Self::MissingBearerToken => (
                StatusCode::UNAUTHORIZED,
                Json(BadRequestErrorDoc {
                    data: json!({ "code": "unauthorized", "message": "missing bearer token" }),
                    errors: Vec::new(),
                    success: false,
                }),
            )
                .into_response(),
        }
    }
}
pub(super) async fn upsert_auth_credential(
    server: &AppState,
    provider_id: &str,
    entry: CredentialEntry,
) -> Result<(), Response> {
    let core = server.core();
    let store = core.store().credentials();
    match store
        .create(
            (provider_id.to_owned(), "default".to_owned()),
            entry.clone(),
        )
        .await
    {
        Ok(_) => Ok(()),
        Err(_) => store
            .update((provider_id.to_owned(), "default".to_owned()), entry)
            .await
            .map(|_| ())
            .map_err(|error| compat_error("store_error", error.to_string())),
    }
}
pub(super) fn compat_error(code: impl Into<String>, message: impl Into<String>) -> Response {
    let code = code.into();
    let message = message.into();
    if code == "not_found" {
        return (
            StatusCode::NOT_FOUND,
            Json(NotFoundErrorDoc {
                name: NotFoundErrorNameDoc::NotFoundError,
                data: NotFoundDataDoc { message },
            }),
        )
            .into_response();
    }
    (
        StatusCode::BAD_REQUEST,
        Json(BadRequestErrorDoc {
            data: json!({ "code": code, "message": message }),
            errors: Vec::new(),
            success: false,
        }),
    )
        .into_response()
}

pub(super) fn require_bearer_token(headers: &HeaderMap) -> Result<(), CompatRequestError> {
    match headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .filter(|value| value.starts_with("Bearer "))
    {
        Some(_) => Ok(()),
        None => Err(CompatRequestError::MissingBearerToken),
    }
}

pub(super) fn invalid_request(code: &str) -> Response {
    compat_error(code, "invalid request")
}
