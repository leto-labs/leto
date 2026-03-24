use futures::StreamExt;
use provider_openai::{
    ChatCompletionChunk, ChatCompletionRequest, Client, Config, Error, ResponseCompactRequest,
    ResponseInputContentPart, ResponseInputItem, ResponseInputRole, ResponseInputTokenCountRequest,
    ResponseItemListRequest, ResponseRequest, ResponseRetrieveRequest, ResponseStream,
    ResponseStreamEvent, ResponseStreamTransport,
};
use tokio::time::{Duration, sleep};

fn smoke_request(prompt: &str) -> ResponseRequest {
    ResponseRequest {
        input: vec![ResponseInputItem::message(
            ResponseInputRole::User,
            vec![ResponseInputContentPart::input_text(prompt)],
        )],
        ..ResponseRequest::default()
    }
}

async fn collect_stream_events(
    mut stream: ResponseStream,
) -> Result<Vec<ResponseStreamEvent>, Error> {
    let mut events = Vec::new();
    while let Some(event) = stream.next().await {
        events.push(event?);
    }
    Ok(events)
}

async fn collect_chat_stream_events(
    mut stream: provider_openai::ChatCompletionStream,
) -> Result<Vec<ChatCompletionChunk>, Error> {
    let mut events = Vec::new();
    while let Some(event) = stream.next().await {
        events.push(event?);
    }
    Ok(events)
}

fn openai_client() -> Option<Client> {
    dotenvy::dotenv().ok();
    std::env::var("OPENAI_API_KEY")
        .ok()
        .map(|api_key| Client::new(Config::new(api_key)))
}

fn is_skippable_live_error(err: &Error) -> bool {
    let message = err.to_string();
    message.contains("401")
        || message.contains("403")
        || message.contains("404")
        || message.contains("429")
        || message.contains("rate limit")
        || message.contains("quota")
}

fn is_optional_resource_error(err: &Error) -> bool {
    let message = err.to_string();
    is_skippable_live_error(err)
        || message.contains("400")
        || message.contains("405")
        || message.contains("409")
        || message.contains("422")
        || message.contains("unsupported")
        || message.contains("not supported")
        || message.contains("unknown endpoint")
}

async fn create_stored_response(client: &Client) -> Result<String, Error> {
    let response = client
        .responses()
        .create(&ResponseRequest {
            store: Some(true),
            ..smoke_request("Say just the word 'hello' and nothing else.")
        })
        .await?;
    response
        .id
        .ok_or_else(|| Error::Inference("stored response did not return an id".into()))
}

#[tokio::test]
async fn create_response_non_streaming() {
    let Some(client) = openai_client() else {
        eprintln!("  SKIP provider-openai create_response: OPENAI_API_KEY not set");
        return;
    };

    let response = match client
        .responses()
        .create(&smoke_request(
            "Say just the word 'hello' and nothing else.",
        ))
        .await
    {
        Ok(response) => response,
        Err(err) if is_skippable_live_error(&err) => {
            eprintln!("  SKIP provider-openai create_response auth/billing: {err}");
            return;
        }
        Err(err) => panic!("provider-openai create_response failed: {err}"),
    };

    assert!(response.id.is_some());
    assert!(response.error.is_none());
}

#[tokio::test]
async fn streams_over_sse() {
    let Some(client) = openai_client() else {
        eprintln!("  SKIP provider-openai sse: OPENAI_API_KEY not set");
        return;
    };

    let stream = match client
        .responses()
        .stream(
            &smoke_request("Say just the word 'hello' and nothing else."),
            ResponseStreamTransport::Sse,
        )
        .await
    {
        Ok(stream) => stream,
        Err(err) if is_skippable_live_error(&err) => {
            eprintln!("  SKIP provider-openai sse auth/billing: {err}");
            return;
        }
        Err(err) => panic!("provider-openai sse failed to start: {err}"),
    };

    let events = collect_stream_events(stream).await.unwrap();
    assert!(events.iter().all(|event| event.sequence_number.is_some()));
}

#[tokio::test]
async fn streams_over_websocket() {
    let Some(client) = openai_client() else {
        eprintln!("  SKIP provider-openai websocket: OPENAI_API_KEY not set");
        return;
    };

    let stream = match client
        .responses()
        .stream(
            &smoke_request("Say just the word 'hello' and nothing else."),
            ResponseStreamTransport::WebSocket,
        )
        .await
    {
        Ok(stream) => stream,
        Err(err) if is_skippable_live_error(&err) => {
            eprintln!("  SKIP provider-openai websocket auth/billing: {err}");
            return;
        }
        Err(err) => panic!("provider-openai websocket failed to start: {err}"),
    };

    let events = collect_stream_events(stream).await.unwrap();
    assert!(events.iter().all(|event| event.sequence_number.is_some()));
}

#[tokio::test]
async fn supports_resource_round_trip() {
    let Some(client) = openai_client() else {
        eprintln!("  SKIP provider-openai resources: OPENAI_API_KEY not set");
        return;
    };

    let response_id = match create_stored_response(&client).await {
        Ok(id) => id,
        Err(err) if is_optional_resource_error(&err) => {
            eprintln!("  SKIP provider-openai resources create/store unsupported: {err}");
            return;
        }
        Err(err) => panic!("provider-openai stored create failed: {err}"),
    };
    sleep(Duration::from_millis(500)).await;

    let retrieved = match client
        .responses()
        .retrieve(&response_id, &ResponseRetrieveRequest::default())
        .await
    {
        Ok(response) => response,
        Err(err) if is_optional_resource_error(&err) => {
            eprintln!("  SKIP provider-openai retrieve/list/delete unsupported: {err}");
            return;
        }
        Err(err) => panic!("provider-openai retrieve failed: {err}"),
    };
    assert_eq!(retrieved.id.as_deref(), Some(response_id.as_str()));

    let input_items = match client
        .responses()
        .list_input_items(&response_id, &ResponseItemListRequest::default())
        .await
    {
        Ok(page) => page,
        Err(err) if is_optional_resource_error(&err) => {
            eprintln!("  SKIP provider-openai input_items unsupported: {err}");
            return;
        }
        Err(err) => panic!("provider-openai input_items failed: {err}"),
    };
    assert!(!input_items.data.is_empty());

    let retrieved_stream = match client
        .responses()
        .stream_retrieve(&response_id, &ResponseRetrieveRequest::default())
        .await
    {
        Ok(stream) => stream,
        Err(err) if is_optional_resource_error(&err) => {
            eprintln!("  SKIP provider-openai stream retrieve unsupported: {err}");
            return;
        }
        Err(err) => panic!("provider-openai stream retrieve failed to start: {err}"),
    };
    let events = collect_stream_events(retrieved_stream).await.unwrap();
    assert!(!events.is_empty());

    match client.responses().delete(&response_id).await {
        Ok(()) => {}
        Err(err) if is_optional_resource_error(&err) => {
            eprintln!("  SKIP provider-openai delete unsupported: {err}");
            return;
        }
        Err(err) => panic!("provider-openai delete failed: {err}"),
    }
}

#[tokio::test]
async fn counts_tokens_and_compacts() {
    let Some(client) = openai_client() else {
        eprintln!("  SKIP provider-openai count/compact: OPENAI_API_KEY not set");
        return;
    };

    let token_count = match client
        .responses()
        .count_input_tokens(&ResponseInputTokenCountRequest {
            input: vec![ResponseInputItem::message(
                ResponseInputRole::User,
                vec![ResponseInputContentPart::input_text("hello world")],
            )],
            ..ResponseInputTokenCountRequest::default()
        })
        .await
    {
        Ok(count) => count,
        Err(err) if is_optional_resource_error(&err) => {
            eprintln!("  SKIP provider-openai input token count unsupported: {err}");
            return;
        }
        Err(err) => panic!("provider-openai input token count failed: {err}"),
    };
    assert!(token_count.input_tokens > 0);

    let compacted = match client
        .responses()
        .compact(&ResponseCompactRequest {
            model: "gpt-5.4-mini".into(),
        })
        .await
    {
        Ok(compacted) => compacted,
        Err(err) if is_optional_resource_error(&err) => {
            eprintln!("  SKIP provider-openai compact unsupported: {err}");
            return;
        }
        Err(err) => panic!("provider-openai compact failed: {err}"),
    };
    assert_eq!(compacted.object.as_deref(), Some("response.compaction"));
}

#[tokio::test]
async fn can_cancel_background_response() {
    let Some(client) = openai_client() else {
        eprintln!("  SKIP provider-openai cancel: OPENAI_API_KEY not set");
        return;
    };

    let created = match client
        .responses()
        .create(&ResponseRequest {
            background: Some(true),
            store: Some(true),
            ..smoke_request("Write a moderately long paragraph about systems programming.")
        })
        .await
    {
        Ok(response) => response,
        Err(err) if is_optional_resource_error(&err) => {
            eprintln!("  SKIP provider-openai background create unsupported: {err}");
            return;
        }
        Err(err) => panic!("provider-openai background create failed: {err}"),
    };

    let Some(response_id) = created.id else {
        eprintln!("  SKIP provider-openai cancel: background response did not return an id");
        return;
    };

    let canceled = match client.responses().cancel(&response_id).await {
        Ok(response) => response,
        Err(err) if is_optional_resource_error(&err) => {
            eprintln!("  SKIP provider-openai cancel unsupported: {err}");
            return;
        }
        Err(err) => panic!("provider-openai cancel failed: {err}"),
    };

    assert_eq!(canceled.id.as_deref(), Some(response_id.as_str()));
}

#[tokio::test]
async fn chat_completions_create_non_streaming() {
    let Some(client) = openai_client() else {
        eprintln!("  SKIP provider-openai chat create: OPENAI_API_KEY not set");
        return;
    };

    let response = match client
        .chat_completions()
        .create(&ChatCompletionRequest::user_text(
            "Say just the word 'hello' and nothing else.",
        ))
        .await
    {
        Ok(response) => response,
        Err(err) if is_skippable_live_error(&err) => {
            eprintln!("  SKIP provider-openai chat create auth/billing: {err}");
            return;
        }
        Err(err) => panic!("provider-openai chat create failed: {err}"),
    };

    assert!(response.id.is_some());
    assert!(!response.choices.is_empty());
}

#[tokio::test]
async fn chat_completions_streams_over_sse() {
    let Some(client) = openai_client() else {
        eprintln!("  SKIP provider-openai chat sse: OPENAI_API_KEY not set");
        return;
    };

    let stream = match client
        .chat_completions()
        .stream(&ChatCompletionRequest::user_text(
            "Say just the word 'hello' and nothing else.",
        ))
        .await
    {
        Ok(stream) => stream,
        Err(err) if is_skippable_live_error(&err) => {
            eprintln!("  SKIP provider-openai chat sse auth/billing: {err}");
            return;
        }
        Err(err) => panic!("provider-openai chat sse failed to start: {err}"),
    };

    let chunks = collect_chat_stream_events(stream).await.unwrap();
    assert!(!chunks.is_empty());
}
