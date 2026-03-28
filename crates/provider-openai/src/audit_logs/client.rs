//! Concrete client for the Audit Logs API surface.
//!
//! Official reference:
//! - List audit logs: <https://platform.openai.com/docs/api-reference/audit-logs>

use crate::Error;
use crate::audit_logs::types::{AuditLogListParams, AuditLogPage};
use crate::client::Client;
use crate::shared::{ensure_success, json_value};

/// Handle for Audit Log operations scoped to a parent [`crate::Client`].
pub struct AuditLogsClient<'a> {
    client: &'a Client,
}

impl<'a> AuditLogsClient<'a> {
    pub(crate) fn new(client: &'a Client) -> Self {
        Self { client }
    }

    /// Lists organization audit-log events.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] if the HTTP request fails or the response cannot be
    /// parsed into an [`AuditLogPage`].
    pub async fn list(&self) -> Result<AuditLogPage, Error> {
        self.list_with_params(&AuditLogListParams::default()).await
    }

    /// Lists organization audit-log events using explicit pagination controls.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] if the HTTP request fails or the response cannot be
    /// parsed into an [`AuditLogPage`].
    pub async fn list_with_params(
        &self,
        params: &AuditLogListParams,
    ) -> Result<AuditLogPage, Error> {
        let response = self
            .client
            .apply_default_headers(
                self.client
                    .http()
                    .get(self.client.endpoint_url("organization/audit_logs"))
                    .query(params)
                    .header("Authorization", self.client.auth_header()),
            )
            .send()
            .await
            .map_err(|e| Error::Inference(e.to_string()))?;

        let response = ensure_success(response).await?;
        serde_json::from_value(json_value(response).await?).map_err(Error::Json)
    }
}
