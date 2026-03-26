use aide::axum::{ApiRouter, routing::get_with};
use axum::extract::Query;

use super::super::types::common::CompatQuery;
use super::super::types::files::{
    FileContentDoc, FileContentTypeDoc, FileDiffStatusDoc, FileDoc, FileNodeDoc, FileNodeTypeDoc,
    FilePathQueryDoc, FindFileQueryDoc, FindSymbolQueryDoc, FindTextMatchDoc, FindTextQueryDoc,
    FindTextSubmatchDoc, PathDoc, SymbolDoc, TextMatchFragmentDoc, VcsInfoDoc,
};
use super::super::*;

pub(super) fn routes() -> ApiRouter<AppState> {
    ApiRouter::new()
        .merge(find_text_route())
        .merge(find_file_route())
        .merge(find_symbol_route())
        .merge(file_list_route())
        .merge(file_content_route())
        .merge(file_status_route())
        .merge(path_get_route())
        .merge(vcs_get_route())
}

fn find_text_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/find",
        get_with(find_text, |operation| {
            operation
                .id("find.text")
                .summary("Find text")
                .description("Search for text patterns across files in the project using ripgrep.")
                .response_with::<200, Json<Vec<FindTextMatchDoc>>, _>(|res| {
                    res.description("Matches")
                })
        }),
    )
}

fn find_file_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/find/file",
        get_with(find_file, |operation| {
            operation
                .id("find.files")
                .summary("Find files")
                .description(
                    "Search for files or directories by name or pattern in the project directory.",
                )
                .response_with::<200, Json<Vec<String>>, _>(|res| res.description("File paths"))
        }),
    )
}

fn find_symbol_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/find/symbol",
        get_with(find_symbol, |operation| {
            operation
                .id("find.symbols")
                .summary("Find symbols")
                .description(
                    "Search for workspace symbols like functions, classes, and variables using LSP.",
                )
                .response_with::<200, Json<Vec<SymbolDoc>>, _>(|res| res.description("Symbols"))
        }),
    )
}

fn file_list_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/file",
        get_with(file_list, |operation| {
            operation
                .id("file.list")
                .summary("List files")
                .description("List files and directories in a specified path.")
                .response_with::<200, Json<Vec<FileNodeDoc>>, _>(|res| {
                    res.description("Files and directories")
                })
        }),
    )
}

fn file_content_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/file/content",
        get_with(file_content, |operation| {
            operation
                .id("file.read")
                .summary("Read file")
                .description("Read the content of a specified file.")
                .response_with::<200, Json<FileContentDoc>, _>(|res| {
                    res.description("File content")
                })
        }),
    )
}

fn file_status_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/file/status",
        get_with(file_status, |operation| {
            operation
                .id("file.status")
                .summary("Get file status")
                .description("Get the git status of all files in the project.")
                .response_with::<200, Json<Vec<FileDoc>>, _>(|res| res.description("File status"))
        }),
    )
}

fn path_get_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/path",
        get_with(path_get, |operation| {
            operation
                .id("path.get")
                .summary("Get paths")
                .description(
                    "Retrieve the current working directory and related path information for the OpenCode instance.",
                )
                .response_with::<200, Json<PathDoc>, _>(|res| res.description("Path"))
        }),
    )
}

fn vcs_get_route() -> ApiRouter<AppState> {
    ApiRouter::new().api_route(
        "/vcs",
        get_with(vcs_get, |operation| {
            operation
                .id("vcs.get")
                .summary("Get VCS info")
                .description(
                    "Retrieve version control system (VCS) information for the current project, such as git branch.",
                )
                .response_with::<200, Json<VcsInfoDoc>, _>(|res| res.description("VCS info"))
        }),
    )
}

async fn find_text(Query(params): Query<FindTextQueryDoc>) -> Response {
    let cwd = query_directory(params.directory.as_deref());
    let output = Command::new("rg")
        .arg("-n")
        .arg("--json")
        .arg("--max-count")
        .arg("10")
        .arg(&params.pattern)
        .arg(&cwd)
        .output();
    match output {
        Ok(output) if output.status.success() || !output.stdout.is_empty() => {
            let matches = String::from_utf8_lossy(&output.stdout)
                .lines()
                .filter_map(|line| serde_json::from_str::<Value>(line).ok())
                .filter(|line| line.get("type").and_then(Value::as_str) == Some("match"))
                .map(|line| FindTextMatchDoc {
                    path: TextMatchFragmentDoc {
                        text: line
                            .pointer("/data/path/text")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_owned(),
                    },
                    lines: TextMatchFragmentDoc {
                        text: line
                            .pointer("/data/lines/text")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_owned(),
                    },
                    line_number: line
                        .pointer("/data/line_number")
                        .and_then(Value::as_f64)
                        .unwrap_or_default(),
                    absolute_offset: line
                        .pointer("/data/absolute_offset")
                        .and_then(Value::as_f64)
                        .unwrap_or_default(),
                    submatches: line
                        .pointer("/data/submatches")
                        .and_then(Value::as_array)
                        .map(|items| {
                            items
                                .iter()
                                .map(|item| FindTextSubmatchDoc {
                                    matched_text: TextMatchFragmentDoc {
                                        text: item
                                            .get("match")
                                            .and_then(|value| value.get("text"))
                                            .and_then(Value::as_str)
                                            .unwrap_or_default()
                                            .to_owned(),
                                    },
                                    start: item
                                        .get("start")
                                        .and_then(Value::as_f64)
                                        .unwrap_or_default(),
                                    end: item
                                        .get("end")
                                        .and_then(Value::as_f64)
                                        .unwrap_or_default(),
                                })
                                .collect()
                        })
                        .unwrap_or_default(),
                })
                .collect::<Vec<_>>();
            Json(matches).into_response()
        }
        _ => Json(Vec::<FindTextMatchDoc>::new()).into_response(),
    }
}

async fn find_file(Query(params): Query<FindFileQueryDoc>) -> Response {
    let query = params.query.to_lowercase();
    let limit = params.limit.unwrap_or(10).clamp(1, 200) as usize;
    let base = query_directory(params.directory.as_deref());
    let mut results = Vec::new();
    collect_paths(&base, &mut results, &query, limit);
    Json(results).into_response()
}

async fn find_symbol(Query(_params): Query<FindSymbolQueryDoc>) -> Response {
    Json(Vec::<SymbolDoc>::new()).into_response()
}

async fn file_list(Query(params): Query<FilePathQueryDoc>) -> Response {
    let path = PathBuf::from(&params.path);
    let path = if path.as_os_str().is_empty() {
        query_directory(params.directory.as_deref())
    } else {
        path
    };
    let mut entries = Vec::new();
    match fs::read_dir(path) {
        Ok(read_dir) => {
            for entry in read_dir.flatten() {
                let absolute = entry.path().display().to_string();
                let file_type = entry.file_type().ok();
                entries.push(FileNodeDoc {
                    path: absolute.clone(),
                    absolute,
                    name: entry.file_name().to_string_lossy().to_string(),
                    node_type: if file_type.as_ref().is_some_and(|kind| kind.is_dir()) {
                        FileNodeTypeDoc::Directory
                    } else {
                        FileNodeTypeDoc::File
                    },
                    ignored: false,
                });
            }
            Json(entries).into_response()
        }
        Err(error) => compat_error("io_error", error.to_string()),
    }
}

async fn file_content(Query(params): Query<FilePathQueryDoc>) -> Response {
    let path = PathBuf::from(&params.path);
    if params.path.is_empty() {
        return invalid_request("missing_path");
    }
    match fs::read_to_string(&path) {
        Ok(content) => Json(FileContentDoc {
            content_type: FileContentTypeDoc::Text,
            content,
            diff: None,
            patch: None,
            encoding: None,
            mime_type: Some(guess_mime(&path).to_owned()),
        })
        .into_response(),
        Err(error) => compat_error("io_error", error.to_string()),
    }
}

async fn file_status(Query(params): Query<CompatQuery>) -> Response {
    let cwd = query_directory(params.directory.as_deref());
    let output = Command::new("git")
        .arg("-C")
        .arg(&cwd)
        .arg("status")
        .arg("--porcelain")
        .output();
    match output {
        Ok(output) if output.status.success() => {
            let payload = String::from_utf8_lossy(&output.stdout)
                .lines()
                .filter(|line| line.len() > 3)
                .map(|line| {
                    let status = line[..2].trim();
                    FileDoc {
                        path: line[3..].to_owned(),
                        added: 0,
                        removed: 0,
                        status: match status.chars().next().unwrap_or('M') {
                            'A' => FileDiffStatusDoc::Added,
                            'D' => FileDiffStatusDoc::Deleted,
                            _ => FileDiffStatusDoc::Modified,
                        },
                    }
                })
                .collect::<Vec<_>>();
            Json(payload).into_response()
        }
        _ => Json(Vec::<FileDoc>::new()).into_response(),
    }
}

async fn path_get(Query(params): Query<CompatQuery>) -> Response {
    let directory = query_directory(params.directory.as_deref());
    Json(PathDoc {
        home: dirs::home_dir()
            .unwrap_or_else(default_directory)
            .display()
            .to_string(),
        state: dirs::state_dir()
            .unwrap_or_else(default_directory)
            .display()
            .to_string(),
        config: dirs::config_dir()
            .unwrap_or_else(default_directory)
            .display()
            .to_string(),
        worktree: directory.display().to_string(),
        directory: directory.display().to_string(),
    })
    .into_response()
}

async fn vcs_get(Query(params): Query<CompatQuery>) -> Response {
    let cwd = query_directory(params.directory.as_deref());
    let output = Command::new("git")
        .arg("-C")
        .arg(&cwd)
        .arg("rev-parse")
        .arg("--abbrev-ref")
        .arg("HEAD")
        .output();
    let branch = output
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        .filter(|value| !value.is_empty());
    Json(VcsInfoDoc {
        branch: branch.unwrap_or_default(),
    })
    .into_response()
}

#[cfg(test)]
mod tests {
    use crate::{
        compat::opencode::test_utils::{
            normalize_opencode_route_doc, opencode_openapi_options, pinned_opencode_openapi,
        },
        utils::openapi::{generate_from_router, subset_for_operations},
    };

    #[test]
    fn find_text_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_opencode_route_doc(&generate_from_router(
                super::find_text_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/find", "get")],
            ))
        );
    }

    #[test]
    fn find_file_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_opencode_route_doc(&generate_from_router(
                super::find_file_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/find/file", "get")],
            ))
        );
    }

    #[test]
    fn find_symbol_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_opencode_route_doc(&generate_from_router(
                super::find_symbol_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/find/symbol", "get")],
            ))
        );
    }

    #[test]
    fn file_list_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_opencode_route_doc(&generate_from_router(
                super::file_list_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/file", "get")],
            ))
        );
    }

    #[test]
    fn file_content_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_opencode_route_doc(&generate_from_router(
                super::file_content_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/file/content", "get")],
            ))
        );
    }

    #[test]
    fn file_status_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_opencode_route_doc(&generate_from_router(
                super::file_status_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/file/status", "get")],
            ))
        );
    }

    #[test]
    fn path_get_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_opencode_route_doc(&generate_from_router(
                super::path_get_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/path", "get")],
            ))
        );
    }

    #[test]
    fn vcs_get_route_openapi_matches_pinned_subset() {
        assert_eq!(
            normalize_opencode_route_doc(&generate_from_router(
                super::vcs_get_route,
                opencode_openapi_options(),
            )),
            normalize_opencode_route_doc(&subset_for_operations(
                &pinned_opencode_openapi(),
                &[("/vcs", "get")],
            ))
        );
    }
}
