pub fn derive_title(prompt: &str) -> String {
    let trimmed = prompt.trim();
    if trimmed.is_empty() {
        return "Codex task".to_string();
    }

    let mut title = trimmed
        .replace("帮我", "")
        .replace("请", "")
        .replace("一下", "")
        .replace("并跑测试", "")
        .replace("并且跑测试", "")
        .replace("然后跑测试", "")
        .trim()
        .to_string();

    for separator in ["。", "，", ",", ".", "\n", " and ", " then "] {
        if let Some((head, _)) = title.split_once(separator) {
            title = head.trim().to_string();
        }
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
}
