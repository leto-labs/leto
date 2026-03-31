#[tokio::test]
async fn canonical_chat_completions_route_returns_non_streaming_completion() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let completion: ChatCompletionObject = client
        .post(format!("{base}/v1/chat/completions"))
        .json(&serde_json::json!({
            "model": "mock-echo",
            "messages": [
                {
                    "role": "user",
                    "content": "hello chat completions"
                }
            ]
        }))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(completion.object.as_deref(), Some("chat.completion"));
    assert_eq!(completion.model.as_deref(), Some("mock-echo"));
    assert_eq!(completion.choices.len(), 1);
    assert_eq!(
        completion.choices[0].message.role,
        provider_openai::ChatCompletionRole::Assistant
    );
    assert!(
        completion.choices[0]
            .message
            .content
            .as_ref()
            .is_some_and(|content| match content {
                provider_openai::ChatCompletionMessageContent::Text(text) => {
                    text.contains("hello chat completions")
                }
                provider_openai::ChatCompletionMessageContent::Parts(_) => false,
            })
    );
}

#[tokio::test]
async fn canonical_chat_completions_route_returns_cache_usage() {
    let base = start_server_with_provider(Arc::new(CacheUsageProvider)).await;
    let client = reqwest::Client::new();

    let completion: ChatCompletionObject = client
        .post(format!("{base}/v1/chat/completions"))
        .json(&serde_json::json!({
            "model": "mock-echo",
            "messages": [
                {
                    "role": "user",
                    "content": "hello cached completions"
                }
            ]
        }))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    let usage = completion
        .usage
        .expect("chat completions should include usage");
    assert_eq!(usage.prompt, 11);
    assert_eq!(usage.completion, 7);
    assert_eq!(usage.total, 18);
    assert_eq!(usage.cache_read, Some(5));
    assert_eq!(usage.cache_write, Some(3));
    assert_eq!(usage.reasoning, Some(2));
}

#[tokio::test]
async fn canonical_chat_completions_route_surfaces_rate_limited_provider_errors() {
    let base = start_server_with_provider(Arc::new(RateLimitedProvider)).await;
    let client = reqwest::Client::new();

    let response = client
        .post(format!("{base}/v1/chat/completions"))
        .json(&serde_json::json!({
            "model": "mock-echo",
            "messages": [
                {
                    "role": "user",
                    "content": "trigger provider rate limit"
                }
            ]
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::BAD_GATEWAY);
    let error: ErrorResponse = response.json().await.unwrap();
    assert_eq!(error.error.code, "runtime_error");
    assert!(error.error.message.contains("429 Too Many Requests"));
    assert!(error.error.message.contains("rate limit exceeded"));
}

#[tokio::test]
async fn canonical_chat_completions_route_surfaces_quota_exceeded_provider_errors() {
    let base = start_server_with_provider(Arc::new(QuotaExceededProvider)).await;
    let client = reqwest::Client::new();

    let response = client
        .post(format!("{base}/v1/chat/completions"))
        .json(&serde_json::json!({
            "model": "mock-echo",
            "messages": [
                {
                    "role": "user",
                    "content": "trigger provider quota exceeded"
                }
            ]
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::BAD_GATEWAY);
    let error: ErrorResponse = response.json().await.unwrap();
    assert_eq!(error.error.code, "runtime_error");
    assert!(error.error.message.contains("429 Too Many Requests"));
    assert!(error.error.message.contains("current quota"));
}

#[tokio::test]
async fn canonical_chat_completions_route_surfaces_throttled_provider_errors() {
    let base = start_server_with_provider(Arc::new(ThrottledProvider)).await;
    let client = reqwest::Client::new();

    let response = client
        .post(format!("{base}/v1/chat/completions"))
        .json(&serde_json::json!({
            "model": "mock-echo",
            "messages": [
                {
                    "role": "user",
                    "content": "trigger provider throttle"
                }
            ]
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::BAD_GATEWAY);
    let error: ErrorResponse = response.json().await.unwrap();
    assert_eq!(error.error.code, "runtime_error");
    assert!(error.error.message.contains("429 Too Many Requests"));
    assert!(error.error.message.contains("request throttled"));
}

#[tokio::test]
async fn canonical_chat_completions_route_surfaces_timeout_provider_errors() {
    let base = start_server_with_provider(Arc::new(TimeoutProvider)).await;
    let client = reqwest::Client::new();

    let response = client
        .post(format!("{base}/v1/chat/completions"))
        .json(&serde_json::json!({
            "model": "mock-echo",
            "messages": [
                {
                    "role": "user",
                    "content": "trigger provider timeout"
                }
            ]
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::BAD_GATEWAY);
    let error: ErrorResponse = response.json().await.unwrap();
    assert_eq!(error.error.code, "runtime_error");
    assert!(error.error.message.contains("timed out"));
}

#[tokio::test]
async fn canonical_audit_logs_route_returns_openai_style_payload() {
    let base = start_server().await;
    let client = Client::new(Config::new("sk-test").with_base_url(format!("{base}/v1")));

    let page: AuditLogPage = client.audit_logs().list().await.unwrap();

    assert_eq!(page.object, "list");
    assert!(!page.has_more);
    assert_eq!(page.data.len(), 2);
    assert_eq!(page.data[0].id, "req_agent_server_20240301");
    assert_eq!(page.data[0].event_type, "api_key.created");
    assert_eq!(page.data[0].effective_at, 1_720_804_090_i64);
    assert_eq!(
        page.data[0]
            .project
            .as_ref()
            .map(|project| project.id.as_str()),
        Some("proj_agent_server")
    );
    assert_eq!(
        page.data[0]
            .actor
            .session
            .as_ref()
            .map(|session| session.user.email.as_str()),
        Some("agent@example.com")
    );
    assert!(page.data[0].extra.contains_key("api_key.created"));
}

#[tokio::test]
async fn canonical_audit_logs_route_supports_pagination_limit() {
    let base = start_server().await;
    let client = Client::new(Config::new("sk-test").with_base_url(format!("{base}/v1")));

    let page: AuditLogPage = client
        .audit_logs()
        .list_with_params(&AuditLogListParams {
            after: None,
            before: None,
            limit: Some(1),
        })
        .await
        .unwrap();

    assert_eq!(page.object, "list");
    assert!(page.has_more);
    assert_eq!(page.data.len(), 1);
    assert_eq!(page.data[0].id, "req_agent_server_20240301");
    assert_eq!(page.data[0].event_type, "api_key.created");
}

#[tokio::test]
async fn canonical_embeddings_route_returns_embedding_vectors() {
    let base = start_server().await;
    let client = Client::new(
        Config::new("sk-test")
            .with_base_url(format!("{base}/v1"))
            .with_model("mock-echo"),
    );

    let response = client
        .embeddings()
        .create(&EmbeddingRequest {
            input: EmbeddingInput::Text("hello embeddings".into()),
            ..EmbeddingRequest::default()
        })
        .await
        .unwrap();

    assert_eq!(response.object, "list");
    assert_eq!(response.model.as_deref(), Some("mock-echo"));
    assert_eq!(response.data[0].object, "embedding");
    assert_eq!(response.data[0].index, 0);

    let embedding = &response.data[0].embedding;
    assert_eq!(embedding.len(), 8);
    assert!(embedding.iter().all(|value| value.is_finite()));

    assert_eq!(
        response.usage.as_ref().map(|usage| usage.prompt_tokens),
        Some(4)
    );
    assert_eq!(
        response.usage.as_ref().map(|usage| usage.total_tokens),
        Some(4)
    );
}

#[tokio::test]
async fn canonical_vector_stores_route_returns_openai_style_payload() {
    let base = start_server().await;
    let client = Client::new(Config::new("sk-test").with_base_url(format!("{base}/v1")));

    let response = client
        .vector_stores()
        .create(&VectorStoreCreateRequest {
            name: Some("  Support FAQ  ".into()),
            description: Some("  Contains support answers  ".into()),
            metadata: Default::default(),
        })
        .await
        .unwrap();

    assert!(response.id.starts_with("vs_"));
    assert_eq!(response.object, "vector_store");
    assert_eq!(response.name.as_deref(), Some("Support FAQ"));
    assert_eq!(
        response.description.as_deref(),
        Some("Contains support answers")
    );
    assert_eq!(response.bytes, Some(0));
    assert_eq!(
        response.file_counts.as_ref().map(|counts| counts.total),
        Some(0)
    );
    assert!(response.created_at.is_some_and(|created_at| created_at > 0));
}

#[tokio::test]
async fn canonical_image_generation_route_returns_base64_images() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let response: serde_json::Value = client
        .post(format!("{base}/v1/images/generations"))
        .json(&serde_json::json!({
            "model": "mock-echo",
            "prompt": "  hello   image  world ",
            "n": 2,
            "response_format": "b64_json"
        }))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    assert!(
        response["created"]
            .as_i64()
            .is_some_and(|created| created > 0)
    );

    let data = response["data"].as_array().unwrap();
    assert_eq!(data.len(), 2);
    assert!(data.iter().all(|item| {
        item["b64_json"]
            .as_str()
            .is_some_and(|image| !image.trim().is_empty())
            && item["revised_prompt"].as_str() == Some("hello image world")
    }));
}

#[tokio::test]
async fn canonical_video_route_returns_openai_style_payload() {
    let base = start_server().await;
    let client = Client::new(
        Config::new("sk-test")
            .with_base_url(format!("{base}/v1"))
            .with_model("mock-echo"),
    );

    let response = client
        .videos()
        .create(&VideoCreateRequest {
            prompt: "a calico cat playing piano".into(),
            seconds: Some("8".into()),
            size: Some("720x1280".into()),
            ..VideoCreateRequest::default()
        })
        .await
        .unwrap();

    assert_eq!(response.object, "video");
    assert_eq!(response.id, "video_agent_server");
    assert_eq!(response.model.as_deref(), Some("mock-echo"));
    assert_eq!(response.status.as_deref(), Some("queued"));
    assert_eq!(response.progress, Some(0));
    assert_eq!(response.seconds.as_deref(), Some("8"));
    assert_eq!(response.size.as_deref(), Some("720x1280"));
    assert_eq!(response.quality.as_deref(), Some("standard"));
    assert!(response.created_at.is_some_and(|created_at| created_at > 0));
}

#[tokio::test]
async fn canonical_moderations_route_returns_openai_style_payload() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let response: serde_json::Value = client
        .post(format!("{base}/v1/moderations"))
        .json(&serde_json::json!({
            "model": "mock-echo",
            "input": "please moderate this text"
        }))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(response["id"].as_str(), Some("modr-agent-server"));
    assert_eq!(response["model"].as_str(), Some("mock-echo"));

    let results = response["results"].as_array().unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0]["flagged"].as_bool(), Some(false));
    assert_eq!(results[0]["categories"]["violence"].as_bool(), Some(false));
    assert_eq!(
        results[0]["category_scores"]["violence"].as_f64(),
        Some(0.0)
    );
}
