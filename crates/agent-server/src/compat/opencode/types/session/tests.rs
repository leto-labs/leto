use serde_json::json;

use super::{CommandPartInputDoc, CommandRequest};

#[test]
fn command_part_input_doc_deserializes_file_payload() {
    let part: CommandPartInputDoc = serde_json::from_value(json!({
        "type": "file",
        "mime": "text/plain",
        "url": "file:///tmp/example.txt"
    }))
    .expect("command file part should deserialize");

    assert!(matches!(part, CommandPartInputDoc::File(_)));
}

#[test]
fn command_request_deserializes_existing_parts_shape() {
    let request: CommandRequest = serde_json::from_value(json!({
        "command": "cat",
        "arguments": "README.md",
        "parts": [
            {
                "type": "file",
                "mime": "text/plain",
                "url": "file:///tmp/example.txt"
            }
        ]
    }))
    .expect("command request should deserialize");

    assert_eq!(request.parts.len(), 1);
    assert!(matches!(request.parts[0], CommandPartInputDoc::File(_)));
}
