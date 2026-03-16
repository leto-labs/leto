use futures::future::BoxFuture;

use brain_types::BrainError;

use super::GrepDriver;

const MAX_MATCHES: usize = 200;

pub struct GrepDriverNative;

impl GrepDriver for GrepDriverNative {
    fn grep(
        &self,
        pattern: &str,
        base_path: Option<&str>,
        include: Option<&str>,
    ) -> BoxFuture<'_, Result<String, BrainError>> {
        let pattern = pattern.to_owned();
        let base_path = base_path.unwrap_or(".").to_owned();
        let include = include.map(|s| s.to_owned());
        Box::pin(async move {
            let re = regex::Regex::new(&pattern).map_err(|e| BrainError::ToolFailed {
                tool: "grep".into(),
                reason: format!("invalid regex: {e}"),
            })?;

            let include_glob = include
                .as_deref()
                .map(|pat| glob::Pattern::new(pat))
                .transpose()
                .map_err(|e| BrainError::ToolFailed {
                    tool: "grep".into(),
                    reason: format!("invalid include pattern: {e}"),
                })?;

            let mut results = Vec::new();
            let mut match_count = 0;

            for entry in walkdir::WalkDir::new(&base_path)
                .follow_links(true)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if !entry.file_type().is_file() {
                    continue;
                }

                let path = entry.path();
                if let Some(ref glob_pat) = include_glob {
                    let file_name = path
                        .file_name()
                        .map(|n| n.to_string_lossy())
                        .unwrap_or_default();
                    if !glob_pat.matches(&file_name) {
                        continue;
                    }
                }

                let content = match std::fs::read_to_string(path) {
                    Ok(c) => c,
                    Err(_) => continue,
                };

                for (line_num, line) in content.lines().enumerate() {
                    if re.is_match(line) {
                        let path_str = path.to_string_lossy();
                        results.push(format!("{}:{}:{}", path_str, line_num + 1, line));
                        match_count += 1;
                        if match_count >= MAX_MATCHES {
                            results.push(format!("... truncated at {MAX_MATCHES} matches"));
                            return Ok(results.join("\n"));
                        }
                    }
                }
            }

            if results.is_empty() {
                Ok("No matches found.".into())
            } else {
                Ok(format!("{match_count} matches:\n{}", results.join("\n")))
            }
        })
    }
}
