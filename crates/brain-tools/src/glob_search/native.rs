use std::path::PathBuf;

use futures::future::BoxFuture;

use brain_types::BrainError;

use super::GlobDriver;

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

            let paths: Vec<String> = glob::glob(&full_pattern)
                .map_err(|e| BrainError::ToolFailed {
                    tool: "glob_search".into(),
                    reason: format!("invalid pattern: {e}"),
                })?
                .filter_map(|entry| entry.ok())
                .map(|p| p.to_string_lossy().into_owned())
                .collect();

            if paths.is_empty() {
                Ok("No files found matching the pattern.".into())
            } else {
                Ok(format!(
                    "{} files found:\n{}",
                    paths.len(),
                    paths.join("\n")
                ))
            }
        })
    }
}
