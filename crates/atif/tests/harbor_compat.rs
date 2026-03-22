use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use atif::{
    Agent, ContentPart, ContentPartKind, ImageMediaType, ImageSource, MessageContent, Observation,
    ObservationResult, SchemaVersion, Step, StepSource, ToolCall, Trajectory,
};
use serde_json::json;
use tempfile::tempdir;

const ENV_FLAG: &str = "ATIF_RUN_HARBOR_COMPAT_TESTS";

#[test]
fn harbor_accepts_valid_minimal_trajectory() {
    run_if_enabled(|| {
        let trajectory = Trajectory {
            schema_version: SchemaVersion::V1_6,
            session_id: "sess_valid_minimal".into(),
            agent: sample_agent(),
            steps: vec![
                Step {
                    step_id: 1,
                    timestamp: Some("2026-03-22T12:00:00Z".into()),
                    source: StepSource::User,
                    model_name: None,
                    reasoning_effort: None,
                    message: "hello".into(),
                    reasoning_content: None,
                    tool_calls: None,
                    observation: None,
                    metrics: None,
                    is_copied_context: None,
                    extra: None,
                },
                Step {
                    step_id: 2,
                    timestamp: Some("2026-03-22T12:00:01Z".into()),
                    source: StepSource::Agent,
                    model_name: Some("gpt-5.4".into()),
                    reasoning_effort: None,
                    message: "done".into(),
                    reasoning_content: None,
                    tool_calls: None,
                    observation: None,
                    metrics: None,
                    is_copied_context: None,
                    extra: None,
                },
            ],
            notes: None,
            final_metrics: None,
            continued_trajectory_ref: None,
            extra: None,
        };

        assert!(trajectory.validate().is_ok());
        run_harbor_validation(&trajectory, true);
    });
}

#[test]
fn harbor_accepts_valid_multimodal_trajectory() {
    run_if_enabled(|| {
        let trajectory = Trajectory {
            schema_version: SchemaVersion::V1_6,
            session_id: "sess_valid_multimodal".into(),
            agent: sample_agent(),
            steps: vec![Step {
                step_id: 1,
                timestamp: Some("2026-03-22T12:00:00Z".into()),
                source: StepSource::User,
                model_name: None,
                reasoning_effort: None,
                message: MessageContent::Parts(vec![
                    ContentPart {
                        kind: ContentPartKind::Text,
                        text: Some("describe this image".into()),
                        source: None,
                    },
                    ContentPart {
                        kind: ContentPartKind::Image,
                        text: None,
                        source: Some(ImageSource {
                            media_type: ImageMediaType::Png,
                            path: "https://example.com/sample.png".into(),
                        }),
                    },
                ]),
                reasoning_content: None,
                tool_calls: None,
                observation: None,
                metrics: None,
                is_copied_context: None,
                extra: None,
            }],
            notes: None,
            final_metrics: None,
            continued_trajectory_ref: None,
            extra: None,
        };

        assert!(trajectory.validate().is_ok());
        run_harbor_validation(&trajectory, true);
    });
}

#[test]
fn harbor_rejects_non_sequential_step_ids() {
    run_if_enabled(|| {
        let trajectory = Trajectory {
            schema_version: SchemaVersion::V1_6,
            session_id: "sess_invalid_steps".into(),
            agent: sample_agent(),
            steps: vec![Step {
                step_id: 2,
                timestamp: Some("2026-03-22T12:00:00Z".into()),
                source: StepSource::User,
                model_name: None,
                reasoning_effort: None,
                message: "hello".into(),
                reasoning_content: None,
                tool_calls: None,
                observation: None,
                metrics: None,
                is_copied_context: None,
                extra: None,
            }],
            notes: None,
            final_metrics: None,
            continued_trajectory_ref: None,
            extra: None,
        };

        assert!(trajectory.validate().is_err());
        run_harbor_validation(&trajectory, false);
    });
}

#[test]
fn harbor_rejects_unknown_tool_call_reference() {
    run_if_enabled(|| {
        let trajectory = Trajectory {
            schema_version: SchemaVersion::V1_6,
            session_id: "sess_invalid_tool_ref".into(),
            agent: sample_agent(),
            steps: vec![Step {
                step_id: 1,
                timestamp: Some("2026-03-22T12:00:00Z".into()),
                source: StepSource::Agent,
                model_name: Some("gpt-5.4".into()),
                reasoning_effort: None,
                message: "Calling file_write".into(),
                reasoning_content: None,
                tool_calls: Some(vec![ToolCall {
                    tool_call_id: "call_1".into(),
                    function_name: "file_write".into(),
                    arguments: serde_json::from_value(json!({
                        "path": "hello.txt",
                        "content": "hello"
                    }))
                    .unwrap(),
                }]),
                observation: Some(Observation {
                    results: vec![ObservationResult {
                        source_call_id: Some("call_2".into()),
                        content: Some("Wrote file".into()),
                        subagent_trajectory_ref: None,
                    }],
                }),
                metrics: None,
                is_copied_context: None,
                extra: None,
            }],
            notes: None,
            final_metrics: None,
            continued_trajectory_ref: None,
            extra: None,
        };

        assert!(trajectory.validate().is_err());
        run_harbor_validation(&trajectory, false);
    });
}

fn run_if_enabled(test: impl FnOnce()) {
    if env::var_os(ENV_FLAG).is_none() {
        eprintln!("skipping Harbor compatibility test; set {ENV_FLAG}=1 to enable");
        return;
    }
    test();
}

fn run_harbor_validation(trajectory: &Trajectory, expect_valid: bool) {
    let temp_dir = tempdir().unwrap();
    let trajectory_path = temp_dir.path().join("trajectory.json");
    fs::write(
        &trajectory_path,
        serde_json::to_vec_pretty(trajectory).unwrap(),
    )
    .unwrap();

    let helper = helper_script_path();
    let harbor_dep = format!("harbor @ file://{}", harbor_repo_path().display());
    let expected = if expect_valid { "valid" } else { "invalid" };

    let output = Command::new("uv")
        .args([
            "run",
            "--with",
            &harbor_dep,
            "python3",
            helper.to_str().unwrap(),
            trajectory_path.to_str().unwrap(),
            expected,
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "Harbor compatibility check failed.\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn sample_agent() -> Agent {
    Agent {
        name: "brain".into(),
        version: "0.1.0".into(),
        model_name: Some("gpt-5.4".into()),
        tool_definitions: None,
        extra: None,
    }
}

fn helper_script_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("py")
        .join("validate_with_harbor.py")
}

fn harbor_repo_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("repocache")
        .join("harbor-framework")
        .join("harbor")
        .canonicalize()
        .unwrap()
}
