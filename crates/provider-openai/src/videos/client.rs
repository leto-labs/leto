//! Concrete client for the Videos API surface.
//!
//! Official reference:
//! - Create a video: <https://platform.openai.com/docs/api-reference/videos/create>

use crate::Error;
use crate::client::Client;
use crate::shared::{ensure_success, json_value};
use crate::videos::types::{VideoCreateRequest, VideoObject};

/// Handle for Video operations scoped to a parent [`crate::Client`].
pub struct VideosClient<'a> {
    client: &'a Client,
}

impl<'a> VideosClient<'a> {
    pub(crate) fn new(client: &'a Client) -> Self {
        Self { client }
    }

    fn normalize_request(&self, request: &mut VideoCreateRequest) {
        if request.model.is_none() {
            request.model = Some(self.client.config().default_model.clone());
        }
    }

    /// Creates a new video-generation job.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] if the HTTP request fails or the response cannot be
    /// parsed into a [`VideoObject`].
    pub async fn create(&self, request: &VideoCreateRequest) -> Result<VideoObject, Error> {
        let mut body = request.clone();
        self.normalize_request(&mut body);

        let mut form = reqwest::multipart::Form::new().text("prompt", body.prompt);
        if let Some(model) = body.model {
            form = form.text("model", model);
        }
        if let Some(seconds) = body.seconds {
            form = form.text("seconds", seconds);
        }
        if let Some(size) = body.size {
            form = form.text("size", size);
        }

        let response = self
            .client
            .apply_default_headers(
                self.client
                    .http()
                    .post(self.client.endpoint_url("videos"))
                    .header("Authorization", self.client.auth_header()),
            )
            .multipart(form)
            .send()
            .await
            .map_err(|e| Error::Inference(e.to_string()))?;

        let response = ensure_success(response).await?;
        serde_json::from_value(json_value(response).await?).map_err(Error::Json)
    }
}
