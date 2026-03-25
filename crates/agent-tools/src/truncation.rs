pub(crate) fn truncate_middle_with_notice(input: &str, max_bytes: usize, label: &str) -> String {
    if input.len() <= max_bytes {
        return input.to_owned();
    }

    let omitted = input.len().saturating_sub(max_bytes);
    let marker = format!("\n...[{label} truncated, omitted approximately {omitted} bytes]...\n");
    if max_bytes <= marker.len() {
        return marker;
    }

    let remaining = max_bytes - marker.len();
    let left_budget = remaining / 2;
    let right_budget = remaining - left_budget;
    let left = safe_prefix(input, left_budget);
    let right = safe_suffix(input, right_budget);
    format!("{left}{marker}{right}")
}

pub(crate) fn truncate_line(input: &str, max_chars: usize) -> String {
    let chars = input.chars().count();
    if chars <= max_chars {
        return input.to_owned();
    }

    let keep = max_chars.saturating_sub(16);
    let prefix: String = input.chars().take(keep).collect();
    format!("{prefix}...[truncated]")
}

fn safe_prefix(input: &str, max_bytes: usize) -> &str {
    if input.len() <= max_bytes {
        return input;
    }
    let mut end = max_bytes.min(input.len());
    while end > 0 && !input.is_char_boundary(end) {
        end -= 1;
    }
    &input[..end]
}

fn safe_suffix(input: &str, max_bytes: usize) -> &str {
    if input.len() <= max_bytes {
        return input;
    }
    let mut start = input.len().saturating_sub(max_bytes);
    while start < input.len() && !input.is_char_boundary(start) {
        start += 1;
    }
    &input[start..]
}
