use agent_runtime::RuntimeError;
use futures::future::BoxFuture;

use super::{FileReadDriver, FileReadRequest, FileReadResponse};
use crate::wasm::{WasmHost, decode_response, encode_request};

/// Wasm-oriented file-read driver backed by a host capability.
pub struct WasmFileReadDriver<H: WasmHost> {
    host: H,
}

impl<H: WasmHost> WasmFileReadDriver<H> {
    /// Creates a wasm-oriented file-read driver backed by the provided host.
    pub fn new(host: H) -> Self {
        Self { host }
    }
}

impl<H: WasmHost> FileReadDriver for WasmFileReadDriver<H> {
    fn read_file(
        &self,
        path: &str,
        offset: Option<usize>,
        limit: Option<usize>,
    ) -> BoxFuture<'_, Result<String, RuntimeError>> {
        let request = FileReadRequest {
            path: path.to_owned(),
            offset,
            limit,
        };
        Box::pin(async move {
            let input = encode_request("agent_tools.file_read", &request)?;
            let output = self.host.call("agent_tools.file_read", input).await?;
            let response = decode_response::<FileReadResponse>("agent_tools.file_read", output)?;
            Ok(response.0)
        })
    }
}

#[cfg(test)]
mod tests {
    use agent_runtime::RuntimeError;
    use futures::future::BoxFuture;
    use serde_json::Value;

    use super::WasmFileReadDriver;
    use crate::file_read::FileReadDriver;
    use crate::wasm::WasmHost;

    struct FakeHost;

    impl WasmHost for FakeHost {
        fn call<'a>(
            &'a self,
            capability: &'static str,
            _input: Value,
        ) -> BoxFuture<'a, Result<Value, RuntimeError>> {
            Box::pin(async move {
                assert_eq!(capability, "agent_tools.file_read");
                Ok(serde_json::json!("     1|hello"))
            })
        }
    }

    #[tokio::test]
    async fn wasm_driver_decodes_host_response() {
        let driver = WasmFileReadDriver::new(FakeHost);
        let output = driver
            .read_file("notes.txt", None, None)
            .await
            .expect("host-backed read should succeed");
        assert_eq!(output, "     1|hello");
    }
}
