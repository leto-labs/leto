use agent_runtime::RuntimeError;
use futures::future::BoxFuture;

use super::{GrepDriver, GrepRequest, GrepResponse};
use crate::wasm::{WasmHost, decode_response, encode_request};

/// Wasm-oriented grep driver backed by a host capability.
pub struct WasmGrepDriver<H: WasmHost> {
    host: H,
}

impl<H: WasmHost> WasmGrepDriver<H> {
    /// Creates a wasm-oriented grep driver backed by the provided host.
    pub fn new(host: H) -> Self {
        Self { host }
    }
}

impl<H: WasmHost> GrepDriver for WasmGrepDriver<H> {
    fn grep(
        &self,
        pattern: &str,
        base_path: Option<&str>,
        include: Option<&str>,
    ) -> BoxFuture<'_, Result<String, RuntimeError>> {
        let request = GrepRequest {
            pattern: pattern.to_owned(),
            path: base_path.map(ToOwned::to_owned),
            include: include.map(ToOwned::to_owned),
        };
        Box::pin(async move {
            let input = encode_request("agent_tools.grep", &request)?;
            let output = self.host.call("agent_tools.grep", input).await?;
            let response = decode_response::<GrepResponse>("agent_tools.grep", output)?;
            Ok(response.0)
        })
    }
}
