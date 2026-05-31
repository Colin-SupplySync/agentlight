use serde::Deserialize;
use std::path::PathBuf;

use crate::confirmation::needs_confirmation;
use crate::domain::NormalizedEvent;

#[cfg_attr(test, allow(dead_code))]
#[derive(Clone, Debug, Default, Deserialize)]
pub struct CodexHookPayload {
    #[serde(default)]
    pub event: String,
    #[serde(default)]
    pub hook_event_name: String,
    #[serde(default)]
    pub session_id: String,
    #[serde(default)]
    pub transcript_path: Option<PathBuf>,
    #[serde(default)]
    pub agent_transcript_path: Option<PathBuf>,
    #[serde(default)]
    pub cwd: String,
    #[serde(default)]
    pub permission_mode: String,
    #[serde(default)]
    pub tool_name: String,
    #[serde(default)]
    pub agent_id: String,
    #[serde(default)]
    pub agent_type: String,
    #[serde(default)]
    pub prompt: String,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub last_assistant_message: Option<String>,
    #[serde(default)]
    pub error: String,
}

pub fn normalize_payload(payload: CodexHookPayload) -> Option<NormalizedEvent> {
    let session_id = payload.session_id.trim();
    if session_id.is_empty() {
        return None;
    }

    let session_id = session_id.to_string();
    let event_name = if payload.hook_event_name.trim().is_empty() {
        payload.event.clone()
    } else {
        payload.hook_event_name.clone()
    };

    match event_name.as_str() {
        "UserPromptSubmit" => Some(NormalizedEvent::UserPromptSubmit {
            session_id,
            prompt: payload.prompt,
        }),
        "PermissionRequest" => Some(NormalizedEvent::PermissionRequested { session_id }),
        "Stop"
            if needs_confirmation(
                payload
                    .last_assistant_message
                    .as_deref()
                    .unwrap_or(&payload.message),
            ) =>
        {
            Some(NormalizedEvent::ConfirmationRequested { session_id })
        }
        "Stop" => Some(NormalizedEvent::Completed { session_id }),
        "Error" | "ToolFailure" | "ConnectionLost" => {
            let reason = if payload.error.trim().is_empty() {
                event_name
            } else {
                payload.error
            };

            Some(NormalizedEvent::Interrupted { session_id, reason })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn payload(event: &str, session_id: &str) -> CodexHookPayload {
        CodexHookPayload {
            event: event.into(),
            session_id: session_id.into(),
            ..Default::default()
        }
    }

    #[test]
    fn maps_user_prompt_submit() {
        let normalized = normalize_payload(CodexHookPayload {
            prompt: "帮我修复测试".into(),
            ..payload("UserPromptSubmit", "s1")
        });

        assert_eq!(
            normalized,
            Some(NormalizedEvent::UserPromptSubmit {
                session_id: "s1".into(),
                prompt: "帮我修复测试".into(),
            })
        );
    }

    #[test]
    fn deserializes_user_prompt_submit_schema_payload() {
        let payload: CodexHookPayload = serde_json::from_str(
            r#"{
                "cwd": "/tmp/project",
                "hook_event_name": "UserPromptSubmit",
                "model": "gpt-5",
                "permission_mode": "default",
                "prompt": "帮我修复真实 payload",
                "session_id": "s1",
                "transcript_path": null,
                "turn_id": "t1"
            }"#,
        )
        .unwrap();

        assert_eq!(
            normalize_payload(payload),
            Some(NormalizedEvent::UserPromptSubmit {
                session_id: "s1".into(),
                prompt: "帮我修复真实 payload".into(),
            })
        );
    }

    #[test]
    fn maps_permission_request() {
        assert_eq!(
            normalize_payload(payload("PermissionRequest", "s1")),
            Some(NormalizedEvent::PermissionRequested {
                session_id: "s1".into(),
            })
        );
    }

    #[test]
    fn deserializes_permission_request_schema_payload() {
        let payload: CodexHookPayload = serde_json::from_str(
            r#"{
                "cwd": "/tmp/project",
                "hook_event_name": "PermissionRequest",
                "model": "gpt-5",
                "permission_mode": "default",
                "session_id": "s1",
                "tool_input": {"cmd": "npm test"},
                "tool_name": "exec_command",
                "transcript_path": null,
                "turn_id": "t1"
            }"#,
        )
        .unwrap();

        assert_eq!(
            normalize_payload(payload),
            Some(NormalizedEvent::PermissionRequested {
                session_id: "s1".into(),
            })
        );
    }

    #[test]
    fn stop_with_confirmation_phrase_requests_confirmation() {
        let normalized = normalize_payload(CodexHookPayload {
            message: "是否继续执行？".into(),
            ..payload("Stop", "s1")
        });

        assert_eq!(
            normalized,
            Some(NormalizedEvent::ConfirmationRequested {
                session_id: "s1".into(),
            })
        );
    }

    #[test]
    fn deserializes_stop_schema_payload_with_last_assistant_message() {
        let payload: CodexHookPayload = serde_json::from_str(
            r#"{
                "cwd": "/tmp/project",
                "hook_event_name": "Stop",
                "last_assistant_message": "是否继续执行下一步？",
                "model": "gpt-5",
                "permission_mode": "default",
                "session_id": "s1",
                "stop_hook_active": false,
                "transcript_path": null,
                "turn_id": "t1"
            }"#,
        )
        .unwrap();

        assert_eq!(
            normalize_payload(payload),
            Some(NormalizedEvent::ConfirmationRequested {
                session_id: "s1".into(),
            })
        );
    }

    #[test]
    fn deserializes_stop_schema_payload_with_null_last_assistant_message() {
        let payload: CodexHookPayload = serde_json::from_str(
            r#"{
                "cwd": "/tmp/project",
                "hook_event_name": "Stop",
                "last_assistant_message": null,
                "model": "gpt-5",
                "permission_mode": "default",
                "session_id": "s1",
                "stop_hook_active": false,
                "transcript_path": null,
                "turn_id": "t1"
            }"#,
        )
        .unwrap();

        assert_eq!(
            normalize_payload(payload),
            Some(NormalizedEvent::Completed {
                session_id: "s1".into(),
            })
        );
    }

    #[test]
    fn stop_without_confirmation_phrase_completes() {
        let normalized = normalize_payload(CodexHookPayload {
            message: "已完成修改并通过测试。".into(),
            ..payload("Stop", "s1")
        });

        assert_eq!(
            normalized,
            Some(NormalizedEvent::Completed {
                session_id: "s1".into(),
            })
        );
    }

    #[test]
    fn empty_session_is_ignored() {
        assert_eq!(normalize_payload(payload("Stop", " ")), None);
    }

    #[test]
    fn error_events_are_interrupted_with_error_reason() {
        let normalized = normalize_payload(CodexHookPayload {
            error: "network unavailable".into(),
            ..payload("ConnectionLost", "s1")
        });

        assert_eq!(
            normalized,
            Some(NormalizedEvent::Interrupted {
                session_id: "s1".into(),
                reason: "network unavailable".into(),
            })
        );
    }

    #[test]
    fn interruption_reason_falls_back_to_event_name() {
        assert_eq!(
            normalize_payload(payload("ToolFailure", "s1")),
            Some(NormalizedEvent::Interrupted {
                session_id: "s1".into(),
                reason: "ToolFailure".into(),
            })
        );
    }
}
