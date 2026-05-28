pub fn needs_confirmation(message: &str) -> bool {
    let normalized = message.to_lowercase();

    [
        "请选择",
        "是否继续",
        "需要确认",
        "你希望",
        "要不要",
        "confirm",
        "approve",
        "proceed",
    ]
    .iter()
    .any(|phrase| normalized.contains(phrase))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_chinese_confirmation_phrases() {
        assert!(needs_confirmation("请选择一个选项继续"));
        assert!(needs_confirmation("是否继续执行下一步？"));
        assert!(needs_confirmation("你希望我现在提交吗？"));
    }

    #[test]
    fn detects_english_confirmation_phrases() {
        assert!(needs_confirmation("Please confirm/proceed before I continue."));
        assert!(needs_confirmation("Waiting for you to approve this change."));
    }

    #[test]
    fn ordinary_completion_does_not_need_confirmation() {
        assert!(!needs_confirmation("已完成修改并通过测试。"));
    }
}
