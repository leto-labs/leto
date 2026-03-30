mod docs;
mod lifecycle;
mod listing;
mod messages;
mod parts;
mod prompts;
#[cfg(test)]
mod tests;

use aide::axum::ApiRouter;

use super::super::AppState;

pub(super) fn routes() -> ApiRouter<AppState> {
    ApiRouter::new()
        .merge(listing::session_list_route())
        .merge(lifecycle::session_create_route())
        .merge(listing::session_status_route())
        .merge(listing::session_get_route())
        .merge(lifecycle::session_update_route())
        .merge(lifecycle::session_delete_route())
        .merge(listing::session_children_route())
        .merge(listing::session_todo_route())
        .merge(listing::session_init_route())
        .merge(lifecycle::session_fork_route())
        .merge(lifecycle::session_abort_route())
        .merge(lifecycle::session_share_route())
        .merge(lifecycle::session_unshare_route())
        .merge(messages::session_diff_route())
        .merge(lifecycle::session_summarize_route())
        .merge(messages::session_messages_route())
        .merge(prompts::session_prompt_route())
        .merge(messages::session_message_route())
        .merge(messages::session_message_delete_route())
        .merge(parts::session_part_update_route())
        .merge(parts::session_part_delete_route())
        .merge(prompts::session_prompt_async_route())
        .merge(prompts::session_command_route())
        .merge(prompts::session_shell_route())
        .merge(lifecycle::session_revert_route())
        .merge(lifecycle::session_unrevert_route())
        .merge(prompts::session_permission_reply_route())
}
