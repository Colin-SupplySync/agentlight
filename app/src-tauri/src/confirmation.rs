pub fn needs_confirmation(message: &str) -> bool {
    let normalized = message.to_lowercase();

    let chinese_phrases = [
        "请选择",
        "是否继续",
        "需要确认",
        "你希望",
        "要不要",
    ];
    let english_request_phrases = [
        "please confirm",
        "please approve",
        "confirm whether",
        "confirm if",
        "approve this",
        "approve the",
        "should i proceed",
        "shall i proceed",
        "before i proceed",
        "before proceeding",
        "proceed?",
    ];

    chinese_phrases
        .iter()
        .chain(english_request_phrases.iter())
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
        assert!(needs_confirmation(
            "Please confirm/proceed before I continue."
        ));
        assert!(needs_confirmation(
            "Waiting for you to approve this change."
        ));
    }

    #[test]
    fn ordinary_completion_does_not_need_confirmation() {
        assert!(!needs_confirmation("已完成修改并通过测试。"));
    }

    #[test]
    fn completed_english_summaries_do_not_need_confirmation() {
        assert!(!needs_confirmation("I confirmed the fix and tests pass."));
        assert!(!needs_confirmation("The change was approved by tests."));
        assert!(!needs_confirmation("I proceeded with the requested update."));
    }
}
