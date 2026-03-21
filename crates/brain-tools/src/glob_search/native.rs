use std::path::PathBuf;

use futures::future::BoxFuture;

use brain_types::BrainError;

use super::GlobDriver;

const MAX_MATCHES: usize = 200;

pub struct GlobDriverNative;

impl GlobDriver for GlobDriverNative {
    fn glob_search(
        &self,
        pattern: &str,
        base_path: Option<&str>,
    ) -> BoxFuture<'_, Result<String, BrainError>> {
        let pattern = pattern.to_owned();
        let base_path = base_path.map(|s| s.to_owned());
        Box::pin(async move {
            let full_pattern = match &base_path {
                Some(base) => {
                    let base = PathBuf::from(base);
                    base.join(&pattern).to_string_lossy().into_owned()
                }
                None => pattern,
            };

            let mut paths: Vec<String> = glob::glob(&full_pattern)
                .map_err(|e| BrainError::ToolFailed {
                    tool: "glob_search".into(),
                    reason: format!("invalid pattern: {e}"),
                })?
                .filter_map(|entry| entry.ok())
                .map(|p| p.to_string_lossy().into_owned())
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
