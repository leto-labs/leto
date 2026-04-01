use agent_runtime::RuntimeError;
use futures::future::BoxFuture;

use super::{ShellDriver, ShellRequest, ShellResponse};
use crate::wasm::{WasmHost, decode_response, encode_request};

/// Wasm-oriented shell driver backed by a host capability.
pub struct WasmShellDriver<H: WasmHost> {
    host: H,
}

impl<H: WasmHost> WasmShellDriver<H> {
    /// Creates a wasm-oriented shell driver backed by the provided host.
    pub fn new(host: H) -> Self {
        Self { host }
    }
}

impl<H: WasmHost> ShellDriver for WasmShellDriver<H> {
    fn run_command(
        &self,
        command: &str,
        working_directory: Option<&str>,
    ) -> BoxFuture<'_, Result<String, RuntimeError>> {
        let request = ShellRequest {
            command: command.to_owned(),
            working_directory: working_directory.map(ToOwned::to_owned),
        };
        Box::pin(async move {
            let input = encode_request("agent_tools.shell", &request)?;
            let output = self.host.call("agent_tools.shell", input).await?;
            let response = decode_response::<ShellResponse>("agent_tools.shell", output)?;
            Ok(response.0)
        })
    }
}
