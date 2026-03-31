#[tokio::test]
async fn canonical_health_route_returns_current_health_payload() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let health: HealthResponse = client
        .get(format!("{base}/v1/health"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    assert!(health.healthy);
    assert_eq!(health.version, env!("CARGO_PKG_VERSION"));
}

#[tokio::test]
async fn root_health_route_returns_current_health_payload() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let health: HealthResponse = client
        .get(format!("{base}/health"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    assert!(health.healthy);
    assert_eq!(health.version, env!("CARGO_PKG_VERSION"));
}

#[tokio::test]
async fn graceful_shutdown_waits_for_in_flight_turn_to_finish() {
    let (base, shutdown_tx, server_task) =
        start_server_with_shutdown(Arc::new(MockProvider::new().with_delay(500))).await;
    let client = reqwest::Client::new();

    let project = create_project(&client, &base).await;
    let session = create_session(&client, &base, &project).await;

    let turn_client = client.clone();
    let turn_base = base.clone();
    let turn_task = tokio::spawn(async move {
        turn_client
            .post(format!("{turn_base}/v1/sessions/{}/turns", session.id))
            .json(&serde_json::json!({
                "input": [
                    {
                        "role": "user",
                        "content": [{"type": "text", "text": "finish before shutdown"}]
                    }
                ]
            }))
            .send()
            .await
            .unwrap()
    });

    tokio::time::sleep(Duration::from_millis(100)).await;
    shutdown_tx.send(()).unwrap();

    let response = tokio::time::timeout(Duration::from_secs(5), turn_task)
        .await
        .unwrap()
        .unwrap();
    assert!(response.status().is_success());

    tokio::time::timeout(Duration::from_secs(5), server_task)
        .await
        .unwrap()
        .unwrap();

    let shutdown_result = client.get(format!("{base}/v1/health")).send().await;
    assert!(shutdown_result.is_err());
}

#[tokio::test]
async fn graceful_shutdown_tears_down_open_event_streams() {
    let (base, shutdown_tx, server_task) =
        start_server_with_shutdown(Arc::new(MockProvider::new())).await;
    let client = reqwest::Client::new();

    let mut response = client
        .get(format!("{base}/v1/events"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    assert!(
        response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("text/event-stream")
    );

    shutdown_tx.send(()).unwrap();

    tokio::time::timeout(Duration::from_secs(5), server_task)
        .await
        .unwrap()
        .unwrap();

    let next_chunk = tokio::time::timeout(Duration::from_secs(5), response.chunk())
        .await
        .unwrap();
    assert!(matches!(next_chunk, Ok(None) | Err(_)));
}

#[tokio::test]
async fn metrics_routes_expose_prometheus_text_payload() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let project = create_project(&client, &base).await;
    let _session = create_session(&client, &base, &project).await;

    for path in ["/metrics", "/v1/metrics"] {
        let response = client
            .get(format!("{base}{path}"))
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap();

        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned();
        let body = response.text().await.unwrap();

        assert!(content_type.starts_with("text/plain"));
        assert!(body.contains("# HELP agent_server_info"));
        assert!(body.contains("agent_server_info{"));
        assert!(body.contains("agent_server_provider_count 1"));
        assert!(body.contains("agent_server_project_count 1"));
        assert!(body.contains("agent_server_session_count 1"));
    }
}

#[tokio::test]
async fn metrics_routes_report_default_labels_and_empty_store_counts() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let response = client
        .get(format!("{base}/metrics"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    let body = response.text().await.unwrap();

    assert!(body.contains("agent_server_info{"));
    assert!(body.contains(r#"default_provider="mock""#));
    assert!(body.contains(r#"default_loop="simple""#));
    assert!(body.contains("agent_server_project_count 0"));
    assert!(body.contains("agent_server_session_count 0"));
}

#[tokio::test]
async fn metrics_routes_export_live_status_snapshot() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let project = create_project(&client, &base).await;
    let _session = create_session(&client, &base, &project).await;

    let status: AgentServerStatus = client
        .get(format!("{base}/v1/status"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    let expected = format!(
        concat!(
            "# HELP agent_server_info Static agent-server build information.\n",
            "# TYPE agent_server_info gauge\n",
            "agent_server_info{{version=\"{}\",default_provider=\"{}\",default_loop=\"{}\"}} 1\n",
            "# HELP agent_server_provider_count Number of configured providers.\n",
            "# TYPE agent_server_provider_count gauge\n",
            "agent_server_provider_count {}\n",
            "# HELP agent_server_loop_count Number of registered loops.\n",
            "# TYPE agent_server_loop_count gauge\n",
            "agent_server_loop_count {}\n",
            "# HELP agent_server_project_count Number of stored projects.\n",
            "# TYPE agent_server_project_count gauge\n",
            "agent_server_project_count {}\n",
            "# HELP agent_server_session_count Number of stored sessions.\n",
            "# TYPE agent_server_session_count gauge\n",
            "agent_server_session_count {}\n"
        ),
        env!("CARGO_PKG_VERSION"),
        status.default_provider_name,
        status.default_loop_name,
        status.provider_names.len(),
        status.loop_names.len(),
        status.project_count,
        status.session_count,
    );

    for path in ["/metrics", "/v1/metrics"] {
        let response = client
            .get(format!("{base}{path}"))
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap();
        let body = response.text().await.unwrap();

        assert_eq!(body, expected);
    }
}

#[tokio::test]
async fn metrics_route_handles_benchmark_style_keep_alive_probes() {
    let server = make_server();
    let router = build_router(server);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });

    let mut connection = BufferedTcpConnection::connect(addr).await;
    let request = format!(
        "GET /metrics HTTP/1.1\r\nhost: {addr}\r\naccept: text/plain\r\nconnection: keep-alive\r\n\r\n"
    );

    for _ in 0..3 {
        connection.send(&request).await;
        let response = connection.read_response().await;
        let body = String::from_utf8(response.body).unwrap();

        assert!(response.status_line.contains("200 OK"));
        assert!(body.contains("agent_server_info{"));
        assert!(body.contains("agent_server_provider_count 1"));
        assert!(body.contains("agent_server_project_count 0"));
        assert!(body.contains("agent_server_session_count 0"));
    }
}

#[tokio::test]
async fn runtime_status_returns_server_status() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let project = create_project(&client, &base).await;
    let _session = create_session(&client, &base, &project).await;

    let status: AgentServerStatus = client
        .get(format!("{base}/v1/status"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(status.provider_names, vec!["mock".to_owned()]);
    assert_eq!(status.default_provider_name, "mock");
    assert_eq!(status.default_loop_name, "simple");
    assert!(status.loop_names.contains(&status.default_loop_name));
    assert_eq!(status.project_count, 1);
    assert_eq!(status.session_count, 1);
}

#[tokio::test]
async fn canonical_agents_route_returns_agent_list() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let response = client
        .get(format!("{base}/v1/agents"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();

    let agents: Vec<AgentInfoRecord> = response.json().await.unwrap();

    assert_eq!(agents, expected_agent_records());
}

#[tokio::test]
async fn canonical_mcp_servers_route_returns_agent_list() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let tools: Vec<AgentInfoRecord> = client
        .get(format!("{base}/v1/mcp/servers"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(tools, expected_agent_records());
}

#[tokio::test]
async fn root_health_route_is_available() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let health = client.get(format!("{base}/health")).send().await.unwrap();
    assert_eq!(health.status(), reqwest::StatusCode::OK);
}

#[tokio::test]
async fn invalid_routes_return_not_found() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let canonical = client
        .get(format!("{base}/v1/does-not-exist"))
        .send()
        .await
        .unwrap();
    assert_eq!(canonical.status(), reqwest::StatusCode::NOT_FOUND);

    let compat = client
        .get(format!("{base}/v1/compat/opencode/does-not-exist"))
        .send()
        .await
        .unwrap();
    assert_eq!(compat.status(), reqwest::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn canonical_project_and_session_routes_round_trip() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let project = create_project(&client, &base).await;
    let session = create_session(&client, &base, &project).await;

    let projects: Vec<Project> = client
        .get(format!("{base}/v1/projects"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(projects.len(), 1);

    let fetched: Session = client
        .get(format!("{base}/v1/sessions/{}", session.id))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(fetched.id, session.id);
}

#[tokio::test]
async fn canonical_project_routes_round_trip_project_config() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let created: Project = client
        .post(format!("{base}/v1/projects"))
        .json(&serde_json::json!({
            "name": "agent-server-config-test",
            "config": {
                "system_prompt": "initial prompt",
                "default_loop": "simple",
                "runtime": {
                    "max_retries": 1,
                    "retry_backoff_ms": 0,
                },
                "default_provider": "mock",
                "default_model": "mock-echo",
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

    assert_eq!(
        created.config.system_prompt.as_deref(),
        Some("initial prompt")
    );
    assert_eq!(created.config.default_loop.as_deref(), Some("simple"));
    assert_eq!(created.config.runtime.max_retries, 1);
    assert_eq!(created.config.runtime.retry_backoff_ms, 0);
    assert_eq!(created.config.default_provider.as_deref(), Some("mock"));
    assert_eq!(created.config.default_model.as_deref(), Some("mock-echo"));

    let updated: Project = client
        .patch(format!("{base}/v1/projects/{}", created.id))
        .json(&serde_json::json!({
            "config": {
                "system_prompt": "reloaded prompt",
                "default_loop": "plan",
                "runtime": {
                    "max_retries": 3,
                    "retry_backoff_ms": 25,
                },
                "default_provider": "mock",
                "default_model": "mock-configured",
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

    assert_eq!(updated.id, created.id);
    assert_eq!(updated.name, created.name);
    assert_eq!(
        updated.config.system_prompt.as_deref(),
        Some("reloaded prompt")
    );
    assert_eq!(updated.config.default_loop.as_deref(), Some("plan"));
    assert_eq!(updated.config.runtime.max_retries, 3);
    assert_eq!(updated.config.runtime.retry_backoff_ms, 25);
    assert_eq!(updated.config.default_provider.as_deref(), Some("mock"));
    assert_eq!(
        updated.config.default_model.as_deref(),
        Some("mock-configured")
    );

    let fetched: Project = client
        .get(format!("{base}/v1/projects/{}", created.id))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(fetched.config, updated.config);
}

#[tokio::test]
async fn canonical_projects_route_returns_empty_list() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let response = client
        .get(format!("{base}/v1/projects"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::OK);

    let projects: Vec<Project> = response.json().await.unwrap();
    assert!(projects.is_empty());
}

#[tokio::test]
async fn canonical_projects_route_lists_created_projects() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let first = create_project(&client, &base).await;
    let second: Project = client
        .post(format!("{base}/v1/projects"))
        .json(&serde_json::json!({"name": "agent-server-test-two"}))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    let projects: Vec<Project> = client
        .get(format!("{base}/v1/projects"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(projects.len(), 2);
    assert!(projects.iter().any(|project| project.id == first.id));
    assert!(projects.iter().any(|project| project.id == second.id));
}

#[tokio::test]
async fn canonical_project_resolve_route_is_idempotent() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let temp_root = env::temp_dir().join(format!(
        "agent-server-project-resolve-{}",
        ulid::Ulid::new()
    ));
    fs::create_dir_all(&temp_root).unwrap();

    let first: Project = client
        .post(format!("{base}/v1/projects/resolve"))
        .json(&serde_json::json!({
            "root": temp_root.join(".").display().to_string()
        }))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    let second: Project = client
        .post(format!("{base}/v1/projects/resolve"))
        .json(&serde_json::json!({
            "root": temp_root.display().to_string()
        }))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(first.id, second.id);
    assert_eq!(first.root, Some(temp_root.clone()));
    assert_eq!(second.root, Some(temp_root.clone()));

    let projects: Vec<Project> = client
        .get(format!("{base}/v1/projects"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].id, first.id);
}

#[tokio::test]
async fn canonical_session_management_routes_cover_project_and_session_reads() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let project = create_project(&client, &base).await;
    let first_session = create_session(&client, &base, &project).await;
    let second_session = create_session(&client, &base, &project).await;

    let fetched_project: Project = client
        .get(format!("{base}/v1/projects/{}", project.id))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(fetched_project.id, project.id);

    let projects: Vec<Project> = client
        .get(format!("{base}/v1/projects"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].id, project.id);

    let project_sessions: Vec<Session> = client
        .get(format!("{base}/v1/projects/{}/sessions", project.id))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(project_sessions.len(), 2);
    assert!(
        project_sessions
            .iter()
            .any(|session| session.id == first_session.id)
    );
    assert!(
        project_sessions
            .iter()
            .any(|session| session.id == second_session.id)
    );

    let sessions: Vec<Session> = client
        .get(format!("{base}/v1/sessions"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(sessions.len(), 2);
    assert!(
        sessions
            .iter()
            .any(|session| session.id == first_session.id)
    );
    assert!(
        sessions
            .iter()
            .any(|session| session.id == second_session.id)
    );

    let fetched_session: Session = client
        .get(format!("{base}/v1/sessions/{}", first_session.id))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(fetched_session.id, first_session.id);
    assert_eq!(fetched_session.project_id, project.id);
}

#[tokio::test]
async fn canonical_session_creation_route_handles_concurrent_requests() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let project = create_project(&client, &base).await;
    let project_id = project.id;
    let request_count = 8;
    let barrier = Arc::new(tokio::sync::Barrier::new(request_count));

    let sessions = join_all((0..request_count).map(|index| {
        let barrier = barrier.clone();
        let client = client.clone();
        let base = base.clone();
        tokio::spawn(async move {
            barrier.wait().await;
            client
                .post(format!("{base}/v1/projects/{project_id}/sessions"))
                .json(&serde_json::json!({
                    "title": format!("concurrent-session-{index}"),
                }))
                .send()
                .await
                .unwrap()
                .error_for_status()
                .unwrap()
                .json::<Session>()
                .await
                .unwrap()
        })
    }))
    .await
    .into_iter()
    .map(|result| result.unwrap())
    .collect::<Vec<_>>();

    let created_session_ids = sessions
        .iter()
        .map(|session| session.id)
        .collect::<BTreeSet<_>>();
    assert_eq!(created_session_ids.len(), request_count);

    let stored_sessions: Vec<Session> = client
        .get(format!("{base}/v1/projects/{project_id}/sessions"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(stored_sessions.len(), request_count);

    let stored_session_ids = stored_sessions
        .iter()
        .map(|session| session.id)
        .collect::<BTreeSet<_>>();
    assert_eq!(stored_session_ids, created_session_ids);
}

#[tokio::test]
async fn canonical_delete_session_route_removes_session_and_related_records() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let project = create_project(&client, &base).await;
    let session = create_session(&client, &base, &project).await;
    let sibling_session = create_session(&client, &base, &project).await;
    let trajectory = sample_trajectory(&session);
    let message = StoredMessage::new(session.id, 0, Message::user_text("delete me"));

    let stored_messages: Vec<StoredMessage> = client
        .put(format!("{base}/v1/sessions/{}/messages", session.id))
        .json(&vec![message.clone()])
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(stored_messages, vec![message]);

    let stored_trajectory: atif::Trajectory = client
        .put(format!("{base}/v1/sessions/{}/trajectory", session.id))
        .json(&trajectory)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(stored_trajectory, trajectory);

    let response = client
        .delete(format!("{base}/v1/sessions/{}", session.id))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::NO_CONTENT);

    let fetch_deleted = client
        .get(format!("{base}/v1/sessions/{}", session.id))
        .send()
        .await
        .unwrap();
    assert_eq!(fetch_deleted.status(), reqwest::StatusCode::NOT_FOUND);
    let error: ErrorResponse = fetch_deleted.json().await.unwrap();
    assert_eq!(error.error.code, "store_not_found");
    assert!(error.error.message.contains(&session.id.to_string()));

    let project_sessions: Vec<Session> = client
        .get(format!("{base}/v1/projects/{}/sessions", project.id))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(project_sessions.len(), 1);
    assert_eq!(project_sessions[0].id, sibling_session.id);

    let messages: Vec<StoredMessage> = client
        .get(format!("{base}/v1/sessions/{}/messages", session.id))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(messages.is_empty());

    let fetched_trajectory: Option<atif::Trajectory> = client
        .get(format!("{base}/v1/sessions/{}/trajectory", session.id))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(fetched_trajectory, None);
}

#[tokio::test]
async fn canonical_trajectory_route_round_trips() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let project = create_project(&client, &base).await;
    let session = create_session(&client, &base, &project).await;
    let trajectory = sample_trajectory(&session);

    let missing: Option<atif::Trajectory> = client
        .get(format!("{base}/v1/sessions/{}/trajectory", session.id))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(missing, None);

    let stored: atif::Trajectory = client
        .put(format!("{base}/v1/sessions/{}/trajectory", session.id))
        .json(&trajectory)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(stored, trajectory);

    let fetched: Option<atif::Trajectory> = client
        .get(format!("{base}/v1/sessions/{}/trajectory", session.id))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(fetched, Some(trajectory));
}

#[tokio::test]
async fn canonical_trajectories_route_lists_stored_trajectories() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let project = create_project(&client, &base).await;
    let first_session = create_session(&client, &base, &project).await;
    let second_session = create_session(&client, &base, &project).await;
    let first_trajectory = sample_trajectory(&first_session);
    let second_trajectory = sample_trajectory(&second_session);

    client
        .put(format!(
            "{base}/v1/sessions/{}/trajectory",
            first_session.id
        ))
        .json(&first_trajectory)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();

    client
        .put(format!(
            "{base}/v1/sessions/{}/trajectory",
            second_session.id
        ))
        .json(&second_trajectory)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();

    let mut trajectories: Vec<TrajectoryRecord> = client
        .get(format!("{base}/v1/trajectories"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    trajectories.sort_by_key(|record| record.session_id);

    let mut expected = vec![
        TrajectoryRecord {
            session_id: first_session.id,
            trajectory: first_trajectory,
        },
        TrajectoryRecord {
            session_id: second_session.id,
            trajectory: second_trajectory,
        },
    ];
    expected.sort_by_key(|record| record.session_id);

    assert_eq!(trajectories, expected);
}

#[tokio::test]
async fn canonical_providers_route_returns_provider_inventory() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let providers: Vec<ProviderCatalogEntry> = client
        .get(format!("{base}/v1/providers"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(providers.len(), 1);
    assert_eq!(providers[0].name, "mock");
    assert_eq!(providers[0].model_ids, vec!["mock-echo".to_owned()]);
}

#[tokio::test]
async fn canonical_health_route_supports_keep_alive_connection_reuse() {
    let server = make_server();
    let router = build_router(server);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });

    let mut connection = BufferedTcpConnection::connect(addr).await;
    let request = format!(
        "GET /v1/health HTTP/1.1\r\nhost: {addr}\r\naccept: application/json\r\nconnection: keep-alive\r\n\r\n"
    );

    connection.send(&request).await;
    let first_response = connection.read_response().await;
    connection.send(&request).await;
    let second_response = connection.read_response().await;

    assert!(first_response.status_line.contains("200 OK"));
    assert!(second_response.status_line.contains("200 OK"));

    let first_health: HealthResponse = serde_json::from_slice(&first_response.body).unwrap();
    let second_health: HealthResponse = serde_json::from_slice(&second_response.body).unwrap();
    assert!(first_health.healthy);
    assert!(second_health.healthy);
    assert_eq!(first_health.version, env!("CARGO_PKG_VERSION"));
    assert_eq!(second_health.version, env!("CARGO_PKG_VERSION"));
}

#[tokio::test]
async fn canonical_project_routes_support_connection_reuse_after_json_post() {
    let server = make_server();
    let router = build_router(server);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });

    let mut connection = BufferedTcpConnection::connect(addr).await;
    let create_project_request = build_json_request(
        "POST",
        "/v1/projects",
        addr,
        r#"{"name":"agent-server-keep-alive-project"}"#,
    );

    connection.send(&create_project_request).await;
    let create_project_response = connection.read_response().await;
    assert!(create_project_response.status_line.contains("201 Created"));

    let project: Project = serde_json::from_slice(&create_project_response.body).unwrap();

    let create_session_request = build_json_request(
        "POST",
        &format!("/v1/projects/{}/sessions", project.id),
        addr,
        "{}",
    );

    connection.send(&create_session_request).await;
    let create_session_response = connection.read_response().await;
    assert!(create_session_response.status_line.contains("201 Created"));

    let session: Session = serde_json::from_slice(&create_session_response.body).unwrap();
    assert_eq!(session.project_id, project.id);
}

#[tokio::test]
async fn canonical_project_routes_recover_connection_reuse_after_bad_json_post() {
    let server = make_server();
    let router = build_router(server);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });

    let mut connection = BufferedTcpConnection::connect(addr).await;
    let malformed_request = build_json_request("POST", "/v1/projects", addr, r#"{"name":"broken""#);

    connection.send(&malformed_request).await;
    let malformed_response = connection.read_response().await;
    assert!(malformed_response.status_line.contains("400 Bad Request"));

    let valid_request = build_json_request(
        "POST",
        "/v1/projects",
        addr,
        r#"{"name":"agent-server-recovery-project"}"#,
    );

    connection.send(&valid_request).await;
    let valid_response = connection.read_response().await;
    assert!(valid_response.status_line.contains("201 Created"));

    let project: Project = serde_json::from_slice(&valid_response.body).unwrap();
    assert_eq!(
        project.name.as_deref(),
        Some("agent-server-recovery-project")
    );
}

#[tokio::test]
async fn canonical_session_routes_recover_connection_reuse_after_bad_json_post() {
    let server = make_server();
    let router = build_router(server);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });

    let mut connection = BufferedTcpConnection::connect(addr).await;
    let create_project_request = build_json_request(
        "POST",
        "/v1/projects",
        addr,
        r#"{"name":"agent-server-session-recovery-project"}"#,
    );

    connection.send(&create_project_request).await;
    let create_project_response = connection.read_response().await;
    assert!(create_project_response.status_line.contains("201 Created"));

    let project: Project = serde_json::from_slice(&create_project_response.body).unwrap();
    let malformed_request = build_json_request(
        "POST",
        &format!("/v1/projects/{}/sessions", project.id),
        addr,
        r#"{"title":"broken""#,
    );

    connection.send(&malformed_request).await;
    let malformed_response = connection.read_response().await;
    assert!(malformed_response.status_line.contains("400 Bad Request"));

    let valid_request = build_json_request(
        "POST",
        &format!("/v1/projects/{}/sessions", project.id),
        addr,
        r#"{"title":"agent-server-recovery-session"}"#,
    );

    connection.send(&valid_request).await;
    let valid_response = connection.read_response().await;
    assert!(valid_response.status_line.contains("201 Created"));

    let session: Session = serde_json::from_slice(&valid_response.body).unwrap();
    assert_eq!(session.project_id, project.id);
    assert_eq!(
        session.title.as_deref(),
        Some("agent-server-recovery-session")
    );
}

#[tokio::test]
async fn canonical_project_routes_preserve_large_request_bodies() {
    let server = make_server();
    let router = build_router(server);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });

    let oversized_name = "request-bloat-".repeat(2_048);
    let create_project_body = serde_json::json!({
        "name": oversized_name,
    })
    .to_string();

    let mut connection = BufferedTcpConnection::connect(addr).await;
    let create_project_request =
        build_json_request("POST", "/v1/projects", addr, &create_project_body);

    connection.send(&create_project_request).await;
    let create_project_response = connection.read_response().await;
    assert!(create_project_response.status_line.contains("201 Created"));

    let project: Project = serde_json::from_slice(&create_project_response.body).unwrap();
    assert!(create_project_body.len() > 16 * 1024);
    assert_eq!(project.name.as_deref(), Some(oversized_name.as_str()));

    let create_session_request = build_json_request(
        "POST",
        &format!("/v1/projects/{}/sessions", project.id),
        addr,
        "{}",
    );

    connection.send(&create_session_request).await;
    let create_session_response = connection.read_response().await;
    assert!(create_session_response.status_line.contains("201 Created"));

    let session: Session = serde_json::from_slice(&create_session_response.body).unwrap();
    assert_eq!(session.project_id, project.id);
}

#[tokio::test]
async fn canonical_credential_health_route_returns_health_records() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let credential = CredentialEntry::api_key("mock-key", "sk-test");
    client
        .post(format!("{base}/v1/credentials/mock/mock-key"))
        .json(&credential)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();

    let mut health = CredentialHealth::default();
    health.record_error("bad gateway", Some("502".into()));
    client
        .patch(format!("{base}/v1/credentials/mock/mock-key/health"))
        .json(&UpdateCredentialHealthRequest {
            health: health.clone(),
        })
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();

    let records: Vec<CredentialHealthRecord> = client
        .get(format!("{base}/v1/credentials/health"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(records.len(), 1);
    assert_eq!(records[0].provider_name, "mock");
    assert_eq!(records[0].credential_id, "mock-key");
    assert_eq!(records[0].health.consecutive_errors, 1);
    assert_eq!(
        records[0]
            .health
            .last_error
            .as_ref()
            .and_then(|error| error.code.as_deref()),
        Some("502")
    );
}

#[tokio::test]
async fn canonical_models_route_returns_provider_inventory() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let models: Vec<ProviderModelRecord> = client
        .get(format!("{base}/v1/models"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(models.len(), 1);
    assert_eq!(models[0].provider_name, "mock");
    assert_eq!(models[0].model.id, "mock-echo");
}

#[tokio::test]
async fn canonical_model_route_returns_model_by_id() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let model: ProviderModelRecord = client
        .get(format!("{base}/v1/models/mock-echo"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(model.provider_name, "mock");
    assert_eq!(model.model.id, "mock-echo");
}

#[tokio::test]
async fn canonical_model_route_returns_not_found_for_unknown_id() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let response = client
        .get(format!("{base}/v1/models/does-not-exist"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::NOT_FOUND);
    let error: ErrorResponse = response.json().await.unwrap();
    assert_eq!(error.error.code, "model_not_found");
}
