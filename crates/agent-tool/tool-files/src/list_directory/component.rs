use agent_runtime::RuntimeError;
use futures::future::BoxFuture;

use super::{ListDirectoryDriver, ListDirectoryRequest, ListDirectoryResponse};
use crate::wasm::{WasmHost, decode_response, encode_request};

/// Wasm-oriented directory-listing driver backed by a host capability.
pub struct WasmListDirectoryDriver<H: WasmHost> {
    host: H,
}

impl<H: WasmHost> WasmListDirectoryDriver<H> {
    /// Creates a wasm-oriented directory-listing driver backed by the provided host.
    pub fn new(host: H) -> Self {
        Self { host }
    }
}

impl<H: WasmHost> ListDirectoryDriver for WasmListDirectoryDriver<H> {
    fn list_directory(
        &self,
        path: &str,
        depth: Option<u32>,
    ) -> BoxFuture<'_, Result<String, RuntimeError>> {
        let request = ListDirectoryRequest {
            path: path.to_owned(),
            depth,
        };
        Box::pin(async move {
            let input = encode_request("agent_tools.list_directory", &request)?;
            let output = self.host.call("agent_tools.list_directory", input).await?;
            let response =
                decode_response::<ListDirectoryResponse>("agent_tools.list_directory", output)?;
            Ok(response.0)
        })
    }
}
