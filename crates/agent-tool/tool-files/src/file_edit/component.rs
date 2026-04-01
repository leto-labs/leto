use agent_runtime::RuntimeError;
use futures::future::BoxFuture;

use super::{FileEditDriver, FileEditRequest, FileEditResponse};
use crate::wasm::{WasmHost, decode_response, encode_request};

/// Wasm-oriented file-edit driver backed by a host capability.
pub struct WasmFileEditDriver<H: WasmHost> {
    host: H,
}

impl<H: WasmHost> WasmFileEditDriver<H> {
    /// Creates a wasm-oriented file-edit driver backed by the provided host.
    pub fn new(host: H) -> Self {
        Self { host }
    }
}

impl<H: WasmHost> FileEditDriver for WasmFileEditDriver<H> {
    fn edit_file(
        &self,
        path: &str,
        old_string: &str,
        new_string: &str,
    ) -> BoxFuture<'_, Result<String, RuntimeError>> {
        let request = FileEditRequest {
            path: path.to_owned(),
            old_string: old_string.to_owned(),
            new_string: new_string.to_owned(),
        };
        Box::pin(async move {
            let input = encode_request("agent_tools.file_edit", &request)?;
            let output = self.host.call("agent_tools.file_edit", input).await?;
            let response = decode_response::<FileEditResponse>("agent_tools.file_edit", output)?;
            Ok(response.0)
        })
    }
}
