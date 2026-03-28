use aide::axum::ApiRouter;

use crate::compat::opencode::AppState;
use crate::compat::opencode::test_utils::{
    normalize_generated_opencode_route_doc, normalize_opencode_route_doc, opencode_openapi_options,
    pinned_opencode_openapi,
};
use crate::utils::openapi::{generate_from_router, subset_for_operations};

fn assert_route_matches(route: fn() -> ApiRouter<AppState>, path: &str, method: &str) {
    assert_eq!(
        normalize_generated_opencode_route_doc(&generate_from_router(
            route,
            opencode_openapi_options()
        )),
        normalize_opencode_route_doc(&subset_for_operations(
            &pinned_opencode_openapi(),
            &[(path, method)],
        ))
    );
}

#[test]
fn session_list_route_openapi_matches_pinned_subset() {
    assert_route_matches(super::listing::session_list_route, "/session", "get");
}

#[test]
fn session_create_route_openapi_matches_pinned_subset() {
    assert_route_matches(super::lifecycle::session_create_route, "/session", "post");
}

#[test]
fn session_status_route_openapi_matches_pinned_subset() {
    assert_route_matches(
        super::listing::session_status_route,
        "/session/status",
        "get",
    );
}

#[test]
fn session_get_route_openapi_matches_pinned_subset() {
    assert_route_matches(
        super::listing::session_get_route,
        "/session/{sessionID}",
        "get",
    );
}

#[test]
fn session_update_route_openapi_matches_pinned_subset() {
    assert_route_matches(
        super::lifecycle::session_update_route,
        "/session/{sessionID}",
        "patch",
    );
}

#[test]
fn session_delete_route_openapi_matches_pinned_subset() {
    assert_route_matches(
        super::lifecycle::session_delete_route,
        "/session/{sessionID}",
        "delete",
    );
}

#[test]
fn session_children_route_openapi_matches_pinned_subset() {
    assert_route_matches(
        super::listing::session_children_route,
        "/session/{sessionID}/children",
        "get",
    );
}

#[test]
fn session_todo_route_openapi_matches_pinned_subset() {
    assert_route_matches(
        super::listing::session_todo_route,
        "/session/{sessionID}/todo",
        "get",
    );
}

#[test]
fn session_init_route_openapi_matches_pinned_subset() {
    assert_route_matches(
        super::listing::session_init_route,
        "/session/{sessionID}/init",
        "post",
    );
}

#[test]
fn session_fork_route_openapi_matches_pinned_subset() {
    assert_route_matches(
        super::lifecycle::session_fork_route,
        "/session/{sessionID}/fork",
        "post",
    );
}

#[test]
fn session_abort_route_openapi_matches_pinned_subset() {
    assert_route_matches(
        super::lifecycle::session_abort_route,
        "/session/{sessionID}/abort",
        "post",
    );
}

#[test]
fn session_share_route_openapi_matches_pinned_subset() {
    assert_route_matches(
        super::lifecycle::session_share_route,
        "/session/{sessionID}/share",
        "post",
    );
}

#[test]
fn session_unshare_route_openapi_matches_pinned_subset() {
    assert_route_matches(
        super::lifecycle::session_unshare_route,
        "/session/{sessionID}/share",
        "delete",
    );
}

#[test]
fn session_diff_route_openapi_matches_pinned_subset() {
    assert_route_matches(
        super::messages::session_diff_route,
        "/session/{sessionID}/diff",
        "get",
    );
}

#[test]
fn session_summarize_route_openapi_matches_pinned_subset() {
    assert_route_matches(
        super::lifecycle::session_summarize_route,
        "/session/{sessionID}/summarize",
        "post",
    );
}

#[test]
fn session_messages_route_openapi_matches_pinned_subset() {
    assert_route_matches(
        super::messages::session_messages_route,
        "/session/{sessionID}/message",
        "get",
    );
}

#[test]
fn session_prompt_route_openapi_matches_pinned_subset() {
    assert_route_matches(
        super::prompts::session_prompt_route,
        "/session/{sessionID}/message",
        "post",
    );
}

#[test]
fn session_message_route_openapi_matches_pinned_subset() {
    assert_route_matches(
        super::messages::session_message_route,
        "/session/{sessionID}/message/{messageID}",
        "get",
    );
}

#[test]
fn session_message_delete_route_openapi_matches_pinned_subset() {
    assert_route_matches(
        super::messages::session_message_delete_route,
        "/session/{sessionID}/message/{messageID}",
        "delete",
    );
}

#[test]
fn session_part_update_route_openapi_matches_pinned_subset() {
    assert_route_matches(
        super::parts::session_part_update_route,
        "/session/{sessionID}/message/{messageID}/part/{partID}",
        "patch",
    );
}

#[test]
fn session_part_delete_route_openapi_matches_pinned_subset() {
    assert_route_matches(
        super::parts::session_part_delete_route,
        "/session/{sessionID}/message/{messageID}/part/{partID}",
        "delete",
    );
}

#[test]
fn session_prompt_async_route_openapi_matches_pinned_subset() {
    assert_route_matches(
        super::prompts::session_prompt_async_route,
        "/session/{sessionID}/prompt_async",
        "post",
    );
}

#[test]
fn session_command_route_openapi_matches_pinned_subset() {
    assert_route_matches(
        super::prompts::session_command_route,
        "/session/{sessionID}/command",
        "post",
    );
}

#[test]
fn session_shell_route_openapi_matches_pinned_subset() {
    assert_route_matches(
        super::prompts::session_shell_route,
        "/session/{sessionID}/shell",
        "post",
    );
}

#[test]
fn session_revert_route_openapi_matches_pinned_subset() {
    assert_route_matches(
        super::lifecycle::session_revert_route,
        "/session/{sessionID}/revert",
        "post",
    );
}

#[test]
fn session_unrevert_route_openapi_matches_pinned_subset() {
    assert_route_matches(
        super::lifecycle::session_unrevert_route,
        "/session/{sessionID}/unrevert",
        "post",
    );
}

#[test]
fn session_permission_reply_route_openapi_matches_pinned_subset() {
    assert_route_matches(
        super::prompts::session_permission_reply_route,
        "/session/{sessionID}/permissions/{permissionID}",
        "post",
    );
}
