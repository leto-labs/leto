#[tokio::test]
async fn compat_auth_endpoints_require_bearer_token() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let provider_auth = client
        .get(format!("{base}/v1/compat/opencode/provider/auth"))
        .send()
        .await
        .unwrap();
    assert_eq!(provider_auth.status(), reqwest::StatusCode::UNAUTHORIZED);

    let oauth_authorize = client
        .post(format!(
            "{base}/v1/compat/opencode/provider/mock/oauth/authorize"
        ))
        .json(&serde_json::json!({ "method": 0 }))
        .send()
        .await
        .unwrap();
    assert_eq!(oauth_authorize.status(), reqwest::StatusCode::UNAUTHORIZED);

    let auth_set = client
        .put(format!("{base}/v1/compat/opencode/auth/mock"))
        .json(&serde_json::json!({
            "type": "api",
            "key": "sk-test"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(auth_set.status(), reqwest::StatusCode::UNAUTHORIZED);

    let mcp_auth = client
        .post(format!("{base}/v1/compat/opencode/mcp/test/auth"))
        .send()
        .await
        .unwrap();
    assert_eq!(mcp_auth.status(), reqwest::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn compat_auth_set_and_remove_round_trip_with_bearer_token() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let auth_set = client
        .put(format!("{base}/v1/compat/opencode/auth/mock"))
        .header(reqwest::header::AUTHORIZATION, "Bearer test-token")
        .json(&serde_json::json!({
            "type": "api",
            "key": "sk-test"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(auth_set.status(), reqwest::StatusCode::OK);
    assert_eq!(auth_set.json::<bool>().await.unwrap(), true);

    let credential: CredentialEntry = client
        .get(format!("{base}/v1/credentials/mock/default"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(credential.id, "mock");
    assert_eq!(credential.label, "mock");
    assert!(credential.enabled);
    match credential.credential {
        ProviderCredential::ApiKey { api_key, .. } => assert_eq!(api_key, "sk-test"),
        other => panic!("expected api key credential, got {other:?}"),
    }

    let auth_remove = client
        .delete(format!("{base}/v1/compat/opencode/auth/mock"))
        .header(reqwest::header::AUTHORIZATION, "Bearer test-token")
        .send()
        .await
        .unwrap();
    assert_eq!(auth_remove.status(), reqwest::StatusCode::OK);
    assert_eq!(auth_remove.json::<bool>().await.unwrap(), true);

    let missing = client
        .get(format!("{base}/v1/credentials/mock/default"))
        .send()
        .await
        .unwrap();
    assert_eq!(missing.status(), reqwest::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn canonical_and_compat_event_endpoints_are_sse() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let canonical = client
        .get(format!("{base}/v1/events"))
        .send()
        .await
        .unwrap();
    assert_eq!(canonical.status(), reqwest::StatusCode::OK);
    assert!(
        canonical
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("text/event-stream")
    );

    let compat = client
        .get(format!("{base}/v1/compat/opencode/global/event"))
        .send()
        .await
        .unwrap();
    assert_eq!(compat.status(), reqwest::StatusCode::OK);
    assert!(
        compat
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("text/event-stream")
    );
}

#[tokio::test]
async fn compat_config_provider_and_prompt_routes_are_real() {
    let base = start_server().await;
    let client = reqwest::Client::new();
    let project = create_project(&client, &base).await;
    let session = create_session(&client, &base, &project).await;

    let health = client
        .get(format!("{base}/v1/compat/opencode/global/health"))
        .send()
        .await
        .unwrap();
    assert_eq!(health.status(), reqwest::StatusCode::OK);

    let config = client
        .get(format!("{base}/v1/compat/opencode/config"))
        .send()
        .await
        .unwrap();
    assert_eq!(config.status(), reqwest::StatusCode::OK);

    let provider_auth = client
        .get(format!("{base}/v1/compat/opencode/provider/auth"))
        .header(reqwest::header::AUTHORIZATION, "Bearer test-token")
        .send()
        .await
        .unwrap();
    assert_eq!(provider_auth.status(), reqwest::StatusCode::OK);

    let prompt = client
        .post(format!(
            "{base}/v1/compat/opencode/session/{}/message",
            session.id
        ))
        .json(&serde_json::json!({
            "parts": [{"type": "text", "text": "hello compat"}]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(prompt.status(), reqwest::StatusCode::OK);

    let prompt_async = client
        .post(format!(
            "{base}/v1/compat/opencode/session/{}/prompt_async",
            session.id
        ))
        .json(&serde_json::json!({
            "parts": [{"type": "text", "text": "hello async compat"}]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(prompt_async.status(), reqwest::StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn compat_config_reads_reflect_latest_patched_values() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let compat_config_patch = serde_json::json!({
        "model": "mock/config-reloaded",
        "share": "auto",
        "username": "compat-user",
        "plugin": ["repo"],
    });
    let compat_config: serde_json::Value = client
        .patch(format!("{base}/v1/compat/opencode/config"))
        .json(&compat_config_patch)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(compat_config, compat_config_patch);

    let reloaded_compat_config: serde_json::Value = client
        .get(format!("{base}/v1/compat/opencode/config"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(reloaded_compat_config, compat_config_patch);

    let initial_global_config: serde_json::Value = client
        .get(format!("{base}/v1/compat/opencode/global/config"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(initial_global_config.get("model").is_none());

    let global_config_patch = serde_json::json!({
        "model": "mock/global-reloaded",
        "default_agent": "plan",
        "username": "global-user",
    });
    let global_config: serde_json::Value = client
        .patch(format!("{base}/v1/compat/opencode/global/config"))
        .json(&global_config_patch)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(global_config, global_config_patch);

    let reloaded_global_config: serde_json::Value = client
        .get(format!("{base}/v1/compat/opencode/global/config"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(reloaded_global_config, global_config_patch);

    let unchanged_compat_config: serde_json::Value = client
        .get(format!("{base}/v1/compat/opencode/config"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(unchanged_compat_config, compat_config_patch);
}

#[tokio::test]
async fn compat_session_lifecycle_routes_are_real() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let temp_root = env::temp_dir().join(format!(
        "agent-server-compat-lifecycle-{}",
        ulid::Ulid::new()
    ));
    fs::create_dir_all(&temp_root).unwrap();
    let directory = temp_root.display().to_string();

    let project = create_project_with_root(&client, &base, &temp_root).await;

    let created: serde_json::Value = client
        .post(format!("{base}/v1/compat/opencode/session"))
        .query(&[("directory", directory.as_str())])
        .json(&serde_json::json!({
            "title": "Lifecycle Root",
            "workspaceID": "wrk-lifecycle",
            "permission": [
                {
                    "permission": "shell",
                    "pattern": "*",
                    "action": "allow"
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
    let session_id = created["id"].as_str().unwrap().to_owned();
    assert_eq!(
        created["projectID"],
        serde_json::json!(project.id.to_string())
    );
    assert_eq!(created["directory"], serde_json::json!(directory));
    assert_eq!(created["title"], serde_json::json!("Lifecycle Root"));
    assert_eq!(created["workspaceID"], serde_json::json!("wrk-lifecycle"));
    assert_eq!(
        created["permission"][0],
        serde_json::json!({
            "permission": "shell",
            "pattern": "*",
            "action": "allow"
        })
    );

    let updated: serde_json::Value = client
        .patch(format!("{base}/v1/compat/opencode/session/{session_id}"))
        .json(&serde_json::json!({
            "title": "Lifecycle Renamed",
            "time": {
                "archived": 1234.0
            }
        }))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(updated["id"], serde_json::json!(session_id));
    assert_eq!(updated["title"], serde_json::json!("Lifecycle Renamed"));
    assert_eq!(updated["time"]["archived"], serde_json::json!(1234.0));

    let shared: serde_json::Value = client
        .post(format!(
            "{base}/v1/compat/opencode/session/{session_id}/share"
        ))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(
        shared["share"]["url"],
        serde_json::json!(format!("https://example.invalid/s/{session_id}"))
    );

    let unshared: serde_json::Value = client
        .delete(format!(
            "{base}/v1/compat/opencode/session/{session_id}/share"
        ))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(unshared.get("share").is_none());

    let fetched: serde_json::Value = client
        .get(format!("{base}/v1/compat/opencode/session/{session_id}"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(fetched["title"], serde_json::json!("Lifecycle Renamed"));
    assert_eq!(fetched["time"]["archived"], serde_json::json!(1234.0));

    let forked: serde_json::Value = client
        .post(format!(
            "{base}/v1/compat/opencode/session/{session_id}/fork"
        ))
        .json(&serde_json::json!({}))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    let forked_id = forked["id"].as_str().unwrap().to_owned();
    assert_ne!(forked_id, session_id);
    assert_eq!(
        forked["projectID"],
        serde_json::json!(project.id.to_string())
    );
    assert_eq!(forked["parentID"], serde_json::json!(session_id));
    assert_eq!(forked["title"], serde_json::json!("Lifecycle Renamed"));

    let children: serde_json::Value = client
        .get(format!(
            "{base}/v1/compat/opencode/session/{session_id}/children"
        ))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(children.as_array().is_some_and(|items| {
        items
            .iter()
            .any(|item| item["id"] == forked_id && item["parentID"] == session_id)
    }));

    let aborted: bool = client
        .post(format!(
            "{base}/v1/compat/opencode/session/{session_id}/abort"
        ))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(aborted);

    let deleted: bool = client
        .delete(format!("{base}/v1/compat/opencode/session/{session_id}"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(deleted);

    let fetch_deleted = client
        .get(format!("{base}/v1/compat/opencode/session/{session_id}"))
        .send()
        .await
        .unwrap();
    assert_eq!(fetch_deleted.status(), reqwest::StatusCode::BAD_REQUEST);

    fs::remove_dir_all(&temp_root).unwrap();
}

#[tokio::test]
async fn compat_dashboard_bootstrap_routes_return_usable_data() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let temp_root =
        env::temp_dir().join(format!("agent-server-compat-warmup-{}", ulid::Ulid::new()));
    fs::create_dir_all(&temp_root).unwrap();
    let directory = temp_root.display().to_string();

    let project = create_project_with_root(&client, &base, &temp_root).await;
    let session = create_session(&client, &base, &project).await;
    let compat_session_id = format!("ses{}", session.id);
    let project_id = project.id.to_string();

    let health: serde_json::Value = client
        .get(format!("{base}/v1/compat/opencode/global/health"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(health["healthy"], serde_json::json!(true));

    let global_path: serde_json::Value = client
        .get(format!("{base}/v1/compat/opencode/path"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(
        global_path["directory"]
            .as_str()
            .is_some_and(|value| !value.is_empty())
    );
    assert_eq!(global_path["directory"], global_path["worktree"]);

    let global_config: serde_json::Value = client
        .get(format!("{base}/v1/compat/opencode/global/config"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(global_config.as_object().is_some());

    let providers: serde_json::Value = client
        .get(format!("{base}/v1/compat/opencode/provider"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(
        providers["all"]
            .as_array()
            .is_some_and(|items| { items.iter().any(|item| item["id"] == "mock") })
    );

    let provider_auth: serde_json::Value = client
        .get(format!("{base}/v1/compat/opencode/provider/auth"))
        .header(
            reqwest::header::AUTHORIZATION,
            "Bearer dashboard-test-token",
        )
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(
        provider_auth["mock"],
        serde_json::json!([
            {
                "type": "api",
                "label": "API Key"
            }
        ])
    );

    let projects: serde_json::Value = client
        .get(format!("{base}/v1/compat/opencode/project"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(projects.as_array().is_some_and(|items| {
        items
            .iter()
            .any(|item| item["id"] == project_id && item["worktree"] == directory)
    }));

    let current_project: serde_json::Value = client
        .get(format!("{base}/v1/compat/opencode/project/current"))
        .query(&[("directory", directory.as_str())])
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(current_project["id"], serde_json::json!(project_id));
    assert_eq!(current_project["worktree"], serde_json::json!(directory));

    let agents: serde_json::Value = client
        .get(format!("{base}/v1/compat/opencode/agent"))
        .query(&[("directory", directory.as_str())])
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(
        agents
            .as_array()
            .is_some_and(|items| items.iter().any(|item| item["name"] == "build"))
    );

    let config: serde_json::Value = client
        .get(format!("{base}/v1/compat/opencode/config"))
        .query(&[("directory", directory.as_str())])
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(config.as_object().is_some());

    let scoped_path: serde_json::Value = client
        .get(format!("{base}/v1/compat/opencode/path"))
        .query(&[("directory", directory.as_str())])
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(scoped_path["directory"], serde_json::json!(directory));
    assert_eq!(scoped_path["worktree"], serde_json::json!(directory));

    let session_status: serde_json::Value = client
        .get(format!("{base}/v1/compat/opencode/session/status"))
        .query(&[("directory", directory.as_str())])
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(
        session_status
            .as_object()
            .is_some_and(|items| items.contains_key(&compat_session_id))
    );

    let vcs: serde_json::Value = client
        .get(format!("{base}/v1/compat/opencode/vcs"))
        .query(&[("directory", directory.as_str())])
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(vcs["branch"].as_str().is_some());

    let commands: serde_json::Value = client
        .get(format!("{base}/v1/compat/opencode/command"))
        .query(&[("directory", directory.as_str())])
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(commands.as_array().is_some());

    let permissions: serde_json::Value = client
        .get(format!("{base}/v1/compat/opencode/permission"))
        .query(&[("directory", directory.as_str())])
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(permissions.as_array().is_some());

    let questions: serde_json::Value = client
        .get(format!("{base}/v1/compat/opencode/question"))
        .query(&[("directory", directory.as_str())])
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(questions.as_array().is_some());

    let sessions: serde_json::Value = client
        .get(format!("{base}/v1/compat/opencode/session"))
        .query(&[("directory", directory.as_str()), ("limit", "20")])
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(sessions.as_array().is_some_and(|items| {
        items
            .iter()
            .any(|item| item["id"] == compat_session_id && item["projectID"] == project_id)
    }));

    let mcp: serde_json::Value = client
        .get(format!("{base}/v1/compat/opencode/mcp"))
        .query(&[("directory", directory.as_str())])
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(mcp.as_object().is_some());

    let lsp: serde_json::Value = client
        .get(format!("{base}/v1/compat/opencode/lsp"))
        .query(&[("directory", directory.as_str())])
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(lsp.as_array().is_some());

    fs::remove_dir_all(&temp_root).unwrap();
}

#[tokio::test]
async fn compat_lsp_route_returns_empty_diagnostics_list() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let lsp: serde_json::Value = client
        .get(format!("{base}/v1/compat/opencode/lsp"))
        .query(&[("directory", env::temp_dir().display().to_string())])
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(lsp, serde_json::json!([]));
}

#[tokio::test]
async fn compat_lsp_route_accepts_workspace_query_and_returns_empty_diagnostics_list() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let lsp: serde_json::Value = client
        .get(format!("{base}/v1/compat/opencode/lsp"))
        .query(&[("workspace", "default")])
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(lsp, serde_json::json!([]));
}

#[tokio::test]
async fn compat_provider_oauth_callback_persists_oauth_credential() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let callback = client
        .post(format!(
            "{base}/v1/compat/opencode/provider/mock/oauth/callback"
        ))
        .header(reqwest::header::AUTHORIZATION, "Bearer test-token")
        .json(&serde_json::json!({
            "method": 0,
            "code": "oauth-code"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(callback.status(), reqwest::StatusCode::OK);
    assert_eq!(callback.json::<bool>().await.unwrap(), true);

    let credential: CredentialEntry = client
        .get(format!("{base}/v1/credentials/mock/default"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(credential.id, "default");
    assert_eq!(credential.label, "mock");
    assert!(credential.enabled);
    match credential.credential {
        ProviderCredential::OAuth(oauth) => {
            assert_eq!(oauth.access_token, "oauth-code");
            assert_eq!(oauth.refresh_token, "compat-refresh");
            assert_eq!(oauth.client_id, "compat-client");
            assert_eq!(oauth.token_endpoint, "https://example.invalid/oauth/token");
            assert_eq!(oauth.token_type.as_deref(), Some("bearer"));
            assert_eq!(oauth.account_id, None);
            assert!(oauth.expires_at > chrono::Utc::now());
        }
        other => panic!("expected oauth credential, got {other:?}"),
    }
}

#[tokio::test]
async fn compat_assistant_endpoint_returns_assistant_message_with_parts() {
    let base = start_server().await;
    let client = reqwest::Client::new();
    let project = create_project(&client, &base).await;
    let session = create_session(&client, &base, &project).await;

    let assistant: serde_json::Value = client
        .post(format!(
            "{base}/v1/compat/opencode/session/{}/message",
            session.id
        ))
        .json(&serde_json::json!({
            "parts": [{"type": "text", "text": "hello compat assistant"}]
        }))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    let assistant_id = assistant["info"]["id"]
        .as_str()
        .expect("compat assistant response should include a message id")
        .to_owned();

    assert_eq!(assistant["info"]["role"], "assistant");
    assert_eq!(assistant["info"]["sessionID"], format!("ses{}", session.id));
    assert_eq!(assistant["info"]["modelID"], "compat");
    assert_eq!(assistant["info"]["providerID"], "compat");
    assert_eq!(assistant["info"]["agent"], "general");
    assert_eq!(assistant["info"]["mode"], "chat");
    assert!(
        assistant_id.starts_with("msg"),
        "compat assistant message ids should use the msg prefix"
    );

    let parts = assistant["parts"]
        .as_array()
        .expect("compat assistant response should include message parts");
    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0]["type"], "text");
    assert_eq!(parts[0]["text"], "hello compat assistant ");
    assert_eq!(parts[0]["sessionID"], format!("ses{}", session.id));
    assert_eq!(parts[0]["messageID"], assistant_id);

    let fetched: serde_json::Value = client
        .get(format!(
            "{base}/v1/compat/opencode/session/{}/message/{}",
            session.id, assistant_id
        ))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(fetched["info"]["id"], assistant["info"]["id"]);
    assert_eq!(fetched["info"]["role"], "assistant");
    assert_eq!(fetched["parts"], assistant["parts"]);
}

#[tokio::test]
async fn compat_thread_endpoints_create_list_fetch_and_fork_sessions() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let temp_root = env::temp_dir().join(format!(
        "agent-server-thread-endpoint-{}",
        ulid::Ulid::new()
    ));
    fs::create_dir_all(&temp_root).unwrap();

    let created: serde_json::Value = client
        .post(format!("{base}/v1/compat/opencode/session"))
        .query(&[("directory", temp_root.display().to_string())])
        .json(&serde_json::json!({
            "title": "Compat thread",
            "workspaceID": "wrk_thread_test"
        }))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    let session_id = created["id"]
        .as_str()
        .expect("compat thread create response should include a session id")
        .to_owned();

    assert!(
        session_id.starts_with("ses"),
        "compat thread session ids should use the ses prefix"
    );
    assert_eq!(created["title"], "Compat thread");
    assert_eq!(created["workspaceID"], "wrk_thread_test");
    assert_eq!(created["directory"], temp_root.display().to_string());
    assert_eq!(created["parentID"], serde_json::Value::Null);

    let listed: Vec<serde_json::Value> = client
        .get(format!("{base}/v1/compat/opencode/session"))
        .query(&[("directory", temp_root.display().to_string())])
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0]["id"], created["id"]);
    assert_eq!(listed[0]["title"], created["title"]);
    assert_eq!(listed[0]["directory"], created["directory"]);

    let fetched: serde_json::Value = client
        .get(format!("{base}/v1/compat/opencode/session/{session_id}"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(fetched["id"], created["id"]);
    assert_eq!(fetched["title"], created["title"]);
    assert_eq!(fetched["workspaceID"], created["workspaceID"]);
    assert_eq!(fetched["directory"], created["directory"]);

    let initialized: bool = client
        .post(format!(
            "{base}/v1/compat/opencode/session/{session_id}/init"
        ))
        .json(&serde_json::json!({
            "modelID": "mock-echo",
            "providerID": "mock",
            "messageID": "msg_init_test"
        }))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    assert!(initialized);

    let forked: serde_json::Value = client
        .post(format!(
            "{base}/v1/compat/opencode/session/{session_id}/fork"
        ))
        .json(&serde_json::json!({}))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_ne!(forked["id"], created["id"]);
    assert_eq!(forked["parentID"], created["id"]);
    assert_eq!(forked["title"], created["title"]);
    assert_eq!(forked["directory"], created["directory"]);

    let children: Vec<serde_json::Value> = client
        .get(format!(
            "{base}/v1/compat/opencode/session/{session_id}/children"
        ))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(children.len(), 1);
    assert_eq!(children[0]["id"], forked["id"]);
    assert_eq!(children[0]["parentID"], created["id"]);
    assert_eq!(children[0]["title"], created["title"]);
}

#[tokio::test]
async fn compat_file_routes_list_directory_and_read_file_content() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let temp_root =
        env::temp_dir().join(format!("agent-server-file-endpoint-{}", ulid::Ulid::new()));
    let nested_dir = temp_root.join("nested");
    let file_path = temp_root.join("notes.txt");
    fs::create_dir_all(&nested_dir).unwrap();
    fs::write(&file_path, "hello from compat file route\n").unwrap();

    let file_list: serde_json::Value = client
        .get(format!("{base}/v1/compat/opencode/file"))
        .query(&[("path", temp_root.display().to_string())])
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    let entries = file_list.as_array().unwrap();
    assert!(entries.iter().any(|entry| {
        entry["name"] == "notes.txt"
            && entry["absolute"] == serde_json::json!(file_path.display().to_string())
            && entry["type"] == "file"
    }));
    assert!(entries.iter().any(|entry| {
        entry["name"] == "nested"
            && entry["absolute"] == serde_json::json!(nested_dir.display().to_string())
            && entry["type"] == "directory"
    }));

    let file_content: serde_json::Value = client
        .get(format!("{base}/v1/compat/opencode/file/content"))
        .query(&[("path", file_path.display().to_string())])
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(file_content["type"], "text");
    assert_eq!(file_content["content"], "hello from compat file route\n");
    assert_eq!(file_content["mimeType"], "text/plain");

    fs::remove_dir_all(&temp_root).unwrap();
}

#[tokio::test]
async fn compat_doc_serves_raw_aide_openapi() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let remote: serde_json::Value = client
        .get(format!("{base}/v1/compat/opencode/doc"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(remote["openapi"], serde_json::json!("3.1.1"));
    assert_eq!(remote["info"]["title"], serde_json::json!("opencode"));
    assert_eq!(
        remote["info"]["description"],
        serde_json::json!("opencode api")
    );
    assert_eq!(remote["info"]["version"], serde_json::json!("0.0.3"));

    let paths = remote["paths"]
        .as_object()
        .expect("compat /doc should return OpenAPI paths");
    assert!(paths.contains_key("/project"));
    assert!(paths.contains_key("/session"));
    assert!(paths.contains_key("/global/health"));
    assert!(!paths.contains_key("/doc"));
    assert!(!paths.contains_key("/v1/compat/opencode/project"));
}

#[test]
fn compat_runtime_does_not_import_pinned_openapi_contract() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/compat/opencode");

    fn scan(path: &Path) {
        for entry in fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                scan(&path);
                continue;
            }
            if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
                continue;
            }
            // `test_utils.rs` is compiled only under `#[cfg(test)]` and is
            // allowed to load the pinned contract for parity tests.
            if path.file_name().and_then(|name| name.to_str()) == Some("test_utils.rs") {
                continue;
            }
            let source = fs::read_to_string(&path).unwrap();
            assert!(
                !source.contains("openapi/opencode.json"),
                "runtime compat source must not reference openapi/opencode.json: {}",
                path.display()
            );
            assert!(
                !source.contains("include_str!(\"../../../../../openapi/opencode.json\")"),
                "runtime compat source must not embed openapi/opencode.json: {}",
                path.display()
            );
        }
    }

    scan(&root);
}

#[tokio::test]
async fn compat_preflight_allows_browser_requests() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let response = client
        .request(
            reqwest::Method::OPTIONS,
            format!("{base}/v1/compat/opencode/config"),
        )
        .header(reqwest::header::ORIGIN, "http://localhost:3000")
        .header(reqwest::header::ACCESS_CONTROL_REQUEST_METHOD, "PATCH")
        .header(
            reqwest::header::ACCESS_CONTROL_REQUEST_HEADERS,
            "authorization,content-type",
        )
        .send()
        .await
        .unwrap();

    assert!(response.status().is_success());
    assert!(
        response
            .headers()
            .get(reqwest::header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .is_some()
    );
}
