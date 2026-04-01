use std::path::PathBuf;

use agent_tool::ToolError;
use futures::future::BoxFuture;

use super::{GlobSearchDriver, MAX_MATCHES};

/// Native filesystem-backed glob search driver.
pub struct NativeGlobSearchDriver;

impl GlobSearchDriver for NativeGlobSearchDriver {
    fn glob_search(
        &self,
        pattern: &str,
        base_path: Option<&str>,
    ) -> BoxFuture<'_, Result<String, ToolError>> {
        let pattern = pattern.to_owned();
        let base_path = base_path.map(ToOwned::to_owned);
        Box::pin(async move {
            let full_pattern = match &base_path {
                Some(base) => PathBuf::from(base)
                    .join(&pattern)
                    .to_string_lossy()
                    .into_owned(),
                None => pattern,
            };

            let mut paths: Vec<String> = glob::glob(&full_pattern)
                .map_err(|error| ToolError::new(format!("invalid glob pattern: {error}")))?
                .filter_map(Result::ok)
                .map(|path| path.to_string_lossy().into_owned())
                .collect();
            paths.sort();

            if paths.is_empty() {
                Ok("No files found matching the pattern.".into())
            } else {
                let total = paths.len();
                let truncated = total > MAX_MATCHES;
                let paths = paths.into_iter().take(MAX_MATCHES).collect::<Vec<_>>();
                let mut output = format!("{total} files found:\n{}", paths.join("\n"));
                if truncated {
                    output.push_str(&format!(
                        "\n... results truncated at {MAX_MATCHES} files; use a narrower pattern or path ..."
                    ));
                }
                Ok(output)
            }
        })
    }
}
