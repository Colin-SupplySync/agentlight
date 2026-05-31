pub fn derive_title(prompt: &str) -> String {
    let trimmed = prompt.trim();
    if trimmed.is_empty() {
        return "Codex task".to_string();
    }

    let mut title = trimmed.to_string();

    for prefix in ["请帮我", "帮我", "请"] {
        if let Some(rest) = title.strip_prefix(prefix) {
            title = rest.trim().to_string();
            break;
        }
    }

    for suffix in ["并且跑测试", "然后跑测试", "并跑测试", "一下"] {
        if let Some(rest) = title.strip_suffix(suffix) {
            title = rest.trim().to_string();
            break;
        }
    }

    for separator in ["。", "，", ",", ".", "\n", " and ", " then "] {
        if let Some((head, _)) = title.split_once(separator) {
            title = head.trim().to_string();
        }
    }

    if is_internal_markdown_heading(&title) {
        return "Codex task".to_string();
    }

    if title.chars().count() > 18 {
        title = title.chars().take(18).collect::<String>();
    }

    if title.is_empty() {
        "Codex task".to_string()
    } else {
        title
    }
}

fn is_internal_markdown_heading(title: &str) -> bool {
    let heading = title.trim().trim_start_matches('#').trim();
    if heading == title.trim() {
        return false;
    }

    matches!(
        heading.to_ascii_lowercase().as_str(),
        "overview" | "agents.md" | "agents" | "instructions" | "system prompt"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_common_prompt_fillers() {
        assert_eq!(
            derive_title("帮我修复移动端导航遮挡并跑测试"),
            "修复移动端导航遮挡"
        );
    }

    #[test]
    fn falls_back_for_empty_prompt() {
        assert_eq!(derive_title("   "), "Codex task");
    }

    #[test]
    fn keeps_meaningful_qing_character_inside_words() {
        assert_eq!(derive_title("申请权限回归测试"), "申请权限回归测试");
    }

    #[test]
    fn ignores_internal_markdown_overview_headings() {
        assert_eq!(derive_title("# Overview"), "Codex task");
        assert_eq!(derive_title("# AGENTS.md"), "Codex task");
    }
}
