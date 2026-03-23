use std::sync::Arc;

use brain_tools::*;
use brain_types::Tool;

#[tokio::test]
async fn echo_tool_returns_message() {
    let tool = EchoTool;
    let def = tool.definition();
    assert_eq!(def.name, "echo");

    let result = tool
        .execute(serde_json::json!({"message": "hello world"}))
        .await
        .unwrap();
    assert_eq!(result, "hello world");
}

#[tokio::test]
async fn echo_tool_empty_message() {
    let tool = EchoTool;
    let result = tool.execute(serde_json::json!({})).await.unwrap();
    assert_eq!(result, "");
}

#[tokio::test]
async fn file_read_writes_and_reads() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.txt");
    std::fs::write(&path, "line1\nline2\nline3\nline4\nline5\n").unwrap();

    let tool = FileReadTool::new(FileReadDriverNative);
    let def = tool.definition();
    assert_eq!(def.name, "file_read");

    let result = tool
        .execute(serde_json::json!({"path": path.to_str().unwrap()}))
        .await
        .unwrap();
    assert!(result.contains("line1"));
    assert!(result.contains("line5"));
}

#[tokio::test]
async fn file_read_with_offset_and_limit() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.txt");
    std::fs::write(&path, "a\nb\nc\nd\ne\n").unwrap();

    let tool = FileReadTool::new(FileReadDriverNative);
    let result = tool
        .execute(serde_json::json!({
            "path": path.to_str().unwrap(),
            "offset": 2,
            "limit": 2
        }))
        .await
        .unwrap();

    assert!(
        result.contains("|b"),
        "expected line 2 content 'b': {result}"
    );
    assert!(
        result.contains("|c"),
        "expected line 3 content 'c': {result}"
    );
    assert!(
        !result.contains("|a"),
        "should not contain line 1: {result}"
    );
    assert!(
        !result.contains("|d"),
        "should not contain line 4: {result}"
    );
}

#[tokio::test]
async fn file_read_adds_continuation_hint_when_more_lines_remain() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.txt");
    let content = (1..=450)
        .map(|line| format!("line-{line}"))
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(&path, content).unwrap();

    let tool = FileReadTool::new(FileReadDriverNative);
    let result = tool
        .execute(serde_json::json!({
            "path": path.to_str().unwrap()
        }))
        .await
        .unwrap();

    assert!(result.contains("Use offset=201 to continue"), "{result}");
}

#[tokio::test]
async fn file_read_missing_path_errors() {
    let tool = FileReadTool::new(FileReadDriverNative);
    let result = tool.execute(serde_json::json!({})).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn file_write_creates_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("sub/dir/output.txt");

    let tool = FileWriteTool::new(FileWriteDriverNative);
    let def = tool.definition();
    assert_eq!(def.name, "file_write");

    let result = tool
        .execute(serde_json::json!({
            "path": path.to_str().unwrap(),
            "content": "hello\nworld"
        }))
        .await
        .unwrap();

    assert!(result.contains("2 lines"));
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "hello\nworld");
}

#[tokio::test]
async fn file_edit_replaces_unique_match() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("edit.txt");
    std::fs::write(&path, "aaa\nbbb\nccc\n").unwrap();

    let tool = FileEditTool::new(FileEditDriverNative);
    let def = tool.definition();
    assert_eq!(def.name, "file_edit");

    tool.execute(serde_json::json!({
        "path": path.to_str().unwrap(),
        "old_string": "bbb",
        "new_string": "BBB"
    }))
    .await
    .unwrap();

    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("BBB"));
    assert!(!content.contains("bbb"));
}

#[tokio::test]
async fn file_edit_rejects_ambiguous() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("dup.txt");
    std::fs::write(&path, "foo\nfoo\nbar\n").unwrap();

    let tool = FileEditTool::new(FileEditDriverNative);
    let result = tool
        .execute(serde_json::json!({
            "path": path.to_str().unwrap(),
            "old_string": "foo",
            "new_string": "baz"
        }))
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn file_edit_rejects_not_found() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nope.txt");
    std::fs::write(&path, "hello\n").unwrap();

    let tool = FileEditTool::new(FileEditDriverNative);
    let result = tool
        .execute(serde_json::json!({
            "path": path.to_str().unwrap(),
            "old_string": "nope_not_here",
            "new_string": "x"
        }))
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn shell_executes_echo() {
    let tool = ShellTool::new(ShellDriverNative);
    let def = tool.definition();
    assert_eq!(def.name, "shell");

    let result = tool
        .execute(serde_json::json!({"command": "echo hello"}))
        .await
        .unwrap();
    assert!(result.contains("hello"));
}

#[tokio::test]
async fn shell_captures_exit_code() {
    let tool = ShellTool::new(ShellDriverNative);
    let result = tool
        .execute(serde_json::json!({"command": "exit 42"}))
        .await
        .unwrap();
    assert!(result.contains("[exit code: 42]"));
}

#[tokio::test]
async fn shell_truncates_large_output() {
    let tool = ShellTool::new(ShellDriverNative);
    let result = tool
        .execute(serde_json::json!({
            "command": "python3 - <<'PY'\nprint('x' * 20000)\nPY"
        }))
        .await
        .unwrap();
    assert!(result.contains("shell output truncated"), "{result}");
}

#[tokio::test]
async fn glob_finds_files() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.rs"), "").unwrap();
    std::fs::write(dir.path().join("b.rs"), "").unwrap();
    std::fs::write(dir.path().join("c.txt"), "").unwrap();

    let tool = GlobTool::new(GlobDriverNative);
    let def = tool.definition();
    assert_eq!(def.name, "glob_search");

    let result = tool
        .execute(serde_json::json!({
            "pattern": "*.rs",
            "path": dir.path().to_str().unwrap()
        }))
        .await
        .unwrap();

    assert!(result.contains("2 files found"));
    assert!(result.contains("a.rs"));
    assert!(result.contains("b.rs"));
    assert!(!result.contains("c.txt"));
}

#[tokio::test]
async fn glob_search_truncates_large_match_sets() {
    let dir = tempfile::tempdir().unwrap();
    for index in 0..250 {
        std::fs::write(dir.path().join(format!("file-{index}.rs")), "").unwrap();
    }

    let tool = GlobTool::new(GlobDriverNative);
    let result = tool
        .execute(serde_json::json!({
            "pattern": "*.rs",
            "path": dir.path().to_str().unwrap()
        }))
        .await
        .unwrap();

    assert!(result.contains("results truncated"), "{result}");
}

#[tokio::test]
async fn grep_finds_matches() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("main.rs"), "fn main() {}\nfn helper() {}\n").unwrap();
    std::fs::write(dir.path().join("lib.rs"), "pub fn lib_fn() {}\n").unwrap();

    let tool = GrepTool::new(GrepDriverNative);
    let def = tool.definition();
    assert_eq!(def.name, "grep");

    let result = tool
        .execute(serde_json::json!({
            "pattern": "fn main",
            "path": dir.path().to_str().unwrap()
        }))
        .await
        .unwrap();

    assert!(result.contains("fn main"));
    assert!(result.contains("main.rs"));
}

#[tokio::test]
async fn grep_with_include_filter() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("code.rs"), "fn hello() {}\n").unwrap();
    std::fs::write(dir.path().join("notes.txt"), "fn hello note\n").unwrap();

    let tool = GrepTool::new(GrepDriverNative);
    let result = tool
        .execute(serde_json::json!({
            "pattern": "fn hello",
            "path": dir.path().to_str().unwrap(),
            "include": "*.rs"
        }))
        .await
        .unwrap();

    assert!(result.contains("code.rs"));
    assert!(!result.contains("notes.txt"));
}

#[tokio::test]
async fn list_directory_truncates_large_directories() {
    let dir = tempfile::tempdir().unwrap();
    for index in 0..250 {
        std::fs::write(dir.path().join(format!("file-{index}.txt")), "").unwrap();
    }

    let tool = ListDirectoryTool::new(ListDirectoryDriverNative);
    let result = tool
        .execute(serde_json::json!({
            "path": dir.path().to_str().unwrap(),
            "depth": 2
        }))
        .await
        .unwrap();

    assert!(result.contains("directory listing truncated"), "{result}");
}

#[tokio::test]
async fn native_tools_returns_all_ten() {
    let tools = native_tools();
    assert_eq!(tools.len(), 10);

    let names: Vec<String> = tools.iter().map(|t| t.definition().name).collect();
    assert!(names.contains(&"echo".to_string()));
    assert!(names.contains(&"file_read".to_string()));
    assert!(names.contains(&"file_write".to_string()));
    assert!(names.contains(&"file_edit".to_string()));
    assert!(names.contains(&"apply_patch".to_string()));
    assert!(names.contains(&"shell".to_string()));
    assert!(names.contains(&"list_directory".to_string()));
    assert!(names.contains(&"glob_search".to_string()));
    assert!(names.contains(&"grep".to_string()));
    assert!(names.contains(&"terminal_session".to_string()));
}

#[tokio::test]
async fn all_tools_are_arc_compatible() {
    let tools = native_tools();
    for tool in &tools {
        let _: Arc<dyn Tool> = tool.clone();
        let def = tool.definition();
        assert!(!def.name.is_empty());
        assert!(!def.description.is_empty());
    }
}
