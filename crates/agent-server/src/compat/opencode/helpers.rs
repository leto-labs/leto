pub(super) async fn clear_compat_state(server: &AppState) {
    server.compat().permissions.write().await.clear();
    server.compat().questions.write().await.clear();
    server.compat().workspaces.write().await.clear();
    server.compat().worktrees.write().await.clear();
    server.compat().mcp_servers.write().await.clear();
    server.compat().ptys.write().await.clear();
    server.compat().tui_requests.write().await.clear();
    server.compat().tui_responses.write().await.clear();
}

pub(super) async fn tui_enqueue(server: &AppState, path: &str, body: Value) {
    server
        .compat()
        .tui_requests
        .write()
        .await
        .push_back(json!({ "path": path, "body": body }));
}
pub(super) fn query_directory(directory: Option<&str>) -> PathBuf {
    directory
        .map(PathBuf::from)
        .unwrap_or_else(default_directory)
}

pub(super) fn collect_paths(base: &FsPath, results: &mut Vec<String>, query: &str, limit: usize) {
    if results.len() >= limit {
        return;
    }
    let Ok(read_dir) = fs::read_dir(base) else {
        return;
    };
    for entry in read_dir.flatten() {
        if results.len() >= limit {
            break;
        }
        let path = entry.path();
        let display = path.display().to_string();
        let name = entry.file_name().to_string_lossy().to_lowercase();
        if query.is_empty() || name.contains(query) {
            results.push(display);
        }
        if entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) {
            collect_paths(&path, results, query, limit);
        }
    }
}

pub(super) fn guess_mime(path: &FsPath) -> &'static str {
    match path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
    {
        "rs" | "ts" | "tsx" | "js" | "json" | "md" | "txt" | "toml" | "yaml" | "yml" => {
            "text/plain"
        }
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "pdf" => "application/pdf",
        _ => "application/octet-stream",
    }
}

pub(super) fn is_git_root(path: &FsPath) -> bool {
    path.join(".git").exists()
}

pub(super) fn json_object(value: &Value) -> BTreeMap<String, Value> {
    value
        .clone()
        .as_object()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .collect()
}

pub(super) fn json_string(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "null".to_owned())
}
