use brain_types::AgentConfig;

pub(crate) fn truncate_tool_result(result: &str, config: &AgentConfig) -> String {
    truncate_middle_with_notice(result, config.tool_output_max_bytes)
}

fn truncate_middle_with_notice(input: &str, max_bytes: usize) -> String {
    if input.len() <= max_bytes {
        return input.to_owned();
    }

    let omitted = input.len().saturating_sub(max_bytes);
    let marker =
        format!("\n...[tool output truncated, omitted approximately {omitted} bytes]...\n");

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leaves_small_outputs_unchanged() {
        let config = AgentConfig::default();
        assert_eq!(truncate_tool_result("ok", &config), "ok");
    }

    #[test]
    fn truncates_large_outputs_with_marker() {
        let config = AgentConfig {
            tool_output_max_bytes: 32,
            ..AgentConfig::default()
        };
        let output = truncate_tool_result("abcdefghijklmnopqrstuvwxyz0123456789", &config);
        assert!(output.contains("tool output truncated"));
    }
}
