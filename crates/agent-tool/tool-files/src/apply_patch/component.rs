use agent_runtime::RuntimeError;
use futures::future::BoxFuture;

use super::{ApplyPatchDriver, ApplyPatchRequest, ApplyPatchResponse};
use crate::wasm::{WasmHost, decode_response, encode_request};

/// Wasm-oriented apply-patch driver backed by a host capability.
pub struct WasmApplyPatchDriver<H: WasmHost> {
    host: H,
}

impl<H: WasmHost> WasmApplyPatchDriver<H> {
    /// Creates a wasm-oriented apply-patch driver backed by the provided host.
    pub fn new(host: H) -> Self {
        Self { host }
    }
}

impl<H: WasmHost> ApplyPatchDriver for WasmApplyPatchDriver<H> {
    fn apply_patch(&self, patch: &str) -> BoxFuture<'_, Result<String, RuntimeError>> {
        let request = ApplyPatchRequest {
            patch: patch.to_owned(),
        };
        Box::pin(async move {
            let input = encode_request("agent_tools.apply_patch", &request)?;
            let output = self.host.call("agent_tools.apply_patch", input).await?;
            let response =
                decode_response::<ApplyPatchResponse>("agent_tools.apply_patch", output)?;
            Ok(response.0)
        })
    }
}
