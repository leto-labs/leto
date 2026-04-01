use provider_openai::parse_webhook_event;

#[test]
fn parses_response_completed_webhook_payload() {
    let payload = serde_json::json!({
        "id": "evt_123",
        "type": "response.completed",
        "created_at": 1_731_000_000_i64,
        "data": {
            "id": "resp_123",
            "object": "response",
            "status": "completed",
            "model": "gpt-4o"
        }
    })
    .to_string();

    let event = parse_webhook_event(payload.as_bytes()).unwrap();

    assert_eq!(event.id, "evt_123");
    assert_eq!(event.event_type, "response.completed");
    assert_eq!(event.created_at, Some(1_731_000_000_i64));
    assert_eq!(
        event.data.get("id").and_then(serde_json::Value::as_str),
        Some("resp_123")
    );
    assert_eq!(
        event.data.get("status").and_then(serde_json::Value::as_str),
        Some("completed")
    );
}
