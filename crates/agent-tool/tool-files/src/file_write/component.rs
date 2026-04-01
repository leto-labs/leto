use agent_runtime::RuntimeError;
use futures::future::BoxFuture;

use super::{FileWriteDriver, FileWriteRequest, FileWriteResponse};
use crate::wasm::{WasmHost, decode_response, encode_request};

/// Wasm-oriented file-write driver backed by a host capability.
pub struct WasmFileWriteDriver<H: WasmHost> {
    host: H,
}

impl<H: WasmHost> WasmFileWriteDriver<H> {
    /// Creates a wasm-oriented file-write driver backed by the provided host.
    pub fn new(host: H) -> Self {
        Self { host }
    }
}

impl<H: WasmHost> FileWriteDriver for WasmFileWriteDriver<H> {
    fn write_file(&self, path: &str, content: &str) -> BoxFuture<'_, Result<String, RuntimeError>> {
        let request = FileWriteRequest {
            path: path.to_owned(),
            content: content.to_owned(),
        };
        Box::pin(async move {
            let input = encode_request("agent_tools.file_write", &request)?;
            let output = self.host.call("agent_tools.file_write", input).await?;
            let response = decode_response::<FileWriteResponse>("agent_tools.file_write", output)?;
            Ok(response.0)
        })
    }
}
