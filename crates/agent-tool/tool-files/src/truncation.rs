#![cfg(feature = "native")]

pub(crate) fn truncate_line(input: &str, max_chars: usize) -> String {
    let chars = input.chars().count();
    if chars <= max_chars {
        return input.to_owned();
    }

    let keep = max_chars.saturating_sub(16);
    let prefix: String = input.chars().take(keep).collect();
    format!("{prefix}...[truncated]")
}
