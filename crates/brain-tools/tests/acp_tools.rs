#![cfg(feature = "acp")]

use std::path::PathBuf;

use agent_client_protocol as acp;
use brain_tools::{
    AcpClientHandle, AcpClientRequest, AcpClientToolContext, FileReadDriverAcp, FileReadTool,
    FileWriteDriverAcp, FileWriteTool, set_active_acp_client_context,
};
use brain_types::Tool;
use tokio::sync::mpsc;

#[tokio::test(flavor = "current_thread")]
async fn acp_file_write_routes_relative_paths_through_client_bridge() {
    let (request_tx, mut request_rx) = mpsc::unbounded_channel();
    let _guard = set_active_acp_client_context(AcpClientToolContext::new(
        AcpClientHandle::new(request_tx),
        acp::SessionId::new("session-1"),
        PathBuf::from("/workspace"),
    ));

    let task = tokio::spawn(async move {
        FileWriteTool::new(FileWriteDriverAcp)
            .execute(serde_json::json!({
                "path": "notes/hello.txt",
                "content": "hello\nworld"
            }))
            .await
    });

    let request = request_rx.recv().await.expect("request should be sent");
    match request {
        AcpClientRequest::WriteTextFile {
            session_id,
            path,
            content,
            response_tx,
        } => {
            assert_eq!(session_id, acp::SessionId::new("session-1"));
            assert_eq!(path, PathBuf::from("/workspace/notes/hello.txt"));
            assert_eq!(content, "hello\nworld");
            let _ = response_tx.send(Ok(()));
        }
        _ => panic!("unexpected ACP request"),
    }

    let result = task
        .await
        .expect("task should join")
        .expect("write should succeed");
    assert!(result.contains("/workspace/notes/hello.txt"));
}

#[tokio::test(flavor = "current_thread")]
async fn acp_file_read_routes_paths_and_formats_output_like_native_driver() {
    let (request_tx, mut request_rx) = mpsc::unbounded_channel();
    let _guard = set_active_acp_client_context(AcpClientToolContext::new(
        AcpClientHandle::new(request_tx),
        acp::SessionId::new("session-2"),
        PathBuf::from("/workspace"),
    ));

    let task = tokio::spawn(async move {
        FileReadTool::new(FileReadDriverAcp)
            .execute(serde_json::json!({
                "path": "notes/hello.txt",
                "offset": 2,
                "limit": 2
            }))
            .await
    });

    let request = request_rx.recv().await.expect("request should be sent");
    match request {
        AcpClientRequest::ReadTextFile {
            session_id,
            path,
            response_tx,
        } => {
            assert_eq!(session_id, acp::SessionId::new("session-2"));
            assert_eq!(path, PathBuf::from("/workspace/notes/hello.txt"));
            let _ = response_tx.send(Ok("a\nb\nc\nd\n".into()));
        }
        _ => panic!("unexpected ACP request"),
    }

    let result = task
        .await
        .expect("task should join")
        .expect("read should succeed");
    assert!(result.contains("|b"), "{result}");
    assert!(result.contains("|c"), "{result}");
    assert!(!result.contains("|a"), "{result}");
    assert!(!result.contains("|d"), "{result}");
}
