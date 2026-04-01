use agent_runtime::RuntimeError;
use futures::future::BoxFuture;

use super::{GlobSearchDriver, GlobSearchRequest, GlobSearchResponse};
use crate::wasm::{WasmHost, decode_response, encode_request};

/// Wasm-oriented glob-search driver backed by a host capability.
pub struct WasmGlobSearchDriver<H: WasmHost> {
    host: H,
}

impl<H: WasmHost> WasmGlobSearchDriver<H> {
    /// Creates a wasm-oriented glob-search driver backed by the provided host.
    pub fn new(host: H) -> Self {
        Self { host }
    }
}

impl<H: WasmHost> GlobSearchDriver for WasmGlobSearchDriver<H> {
    fn glob_search(
        &self,
        pattern: &str,
        base_path: Option<&str>,
    ) -> BoxFuture<'_, Result<String, RuntimeError>> {
        let request = GlobSearchRequest {
            pattern: pattern.to_owned(),
            path: base_path.map(ToOwned::to_owned),
        };
        Box::pin(async move {
            let input = encode_request("agent_tools.glob_search", &request)?;
            let output = self.host.call("agent_tools.glob_search", input).await?;
            let response =
                decode_response::<GlobSearchResponse>("agent_tools.glob_search", output)?;
            Ok(response.0)
        })
    }
}
