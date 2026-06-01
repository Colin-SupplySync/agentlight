#[cfg(not(test))]
use std::fs::{self, OpenOptions};
use std::future::Future;
#[cfg(not(test))]
use std::io::Write;
use std::sync::{Arc, Mutex};
#[cfg(not(test))]
use std::time::{SystemTime, UNIX_EPOCH};

use axum::{extract::State, http::StatusCode, routing::post, Json, Router};

use crate::codex_provider::{normalize_payload, CodexHookPayload};
use crate::codex_sessions::{
    session_meta_for_session, session_meta_from_transcript, thread_name_for_session,
    CodexSessionMeta, CodexThreadSource,
};
use crate::domain::NormalizedEvent;
use crate::notifier::{BarkNotifier, OutboundNotification};
use crate::settings::AppSettings;
use crate::task_store::{NotificationRequest, TaskStore};
use crate::title::derive_title;

#[derive(Clone)]
pub struct SharedBackendState {
    pub store: Arc<Mutex<TaskStore>>,
    pub settings: Arc<Mutex<AppSettings>>,
}

impl SharedBackendState {
    pub fn new(store: TaskStore, settings: AppSettings) -> Self {
        Self {
            store: Arc::new(Mutex::new(store)),
            settings: Arc::new(Mutex::new(settings)),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HookAction {
    Shown,
    FoldedToParent,
    IgnoredBypassedPermission,
    IgnoredLowSignalPrompt,
    IgnoredSubagentLifecycle,
    IgnoredSubagentWithoutParent,
    IgnoredUntrustedTask,
    IgnoredUnknownEvent,
}

impl HookAction {
    #[cfg_attr(test, allow(dead_code))]
    fn as_str(self) -> &'static str {
        match self {
            HookAction::Shown => "shown",
            HookAction::FoldedToParent => "folded_to_parent",
            HookAction::IgnoredBypassedPermission => "ignored_bypassed_permission",
            HookAction::IgnoredLowSignalPrompt => "ignored_low_signal_prompt",
            HookAction::IgnoredSubagentLifecycle => "ignored_subagent_lifecycle",
            HookAction::IgnoredSubagentWithoutParent => "ignored_subagent_without_parent",
            HookAction::IgnoredUntrustedTask => "ignored_untrusted_task",
            HookAction::IgnoredUnknownEvent => "ignored_unknown_event",
        }
    }
}

struct HookDecision {
    event: NormalizedEvent,
    title_hint: Option<String>,
    action: HookAction,
    display_task_id: String,
    meta: Option<CodexSessionMeta>,
}

enum HookDecisionOutcome {
    Apply(HookDecision),
    Ignore {
        action: HookAction,
        display_task_id: Option<String>,
        meta: Option<CodexSessionMeta>,
    },
}

pub fn router(state: SharedBackendState) -> Router {
    Router::new()
        .route("/codex-hook", post(handle_codex_hook))
        .with_state(state)
}

async fn maybe_send_bark_notification<F, Fut>(
    notification: NotificationRequest,
    settings: AppSettings,
    send: F,
) where
    F: FnOnce(OutboundNotification) -> Fut,
    Fut: Future<Output = Result<(), String>>,
{
    if !settings.notifications_enabled || settings.bark_endpoint_url.trim().is_empty() {
        return;
    }

    let outbound = OutboundNotification {
        status: notification.status,
        title: notification.title,
    };

    if let Err(err) = send(outbound).await {
        eprintln!("failed to send Bark notification: {err}");
    }
}

fn notification_for_payload(
    state: &SharedBackendState,
    payload: CodexHookPayload,
) -> Option<(NotificationRequest, AppSettings)> {
    notification_for_payload_with_title(state, payload, None)
}

fn notification_for_payload_with_title(
    state: &SharedBackendState,
    payload: CodexHookPayload,
    title_hint: Option<String>,
) -> Option<(NotificationRequest, AppSettings)> {
    let Some(event) = normalize_payload(payload.clone()) else {
        append_hook_event_log(&payload, None, HookAction::IgnoredUnknownEvent, None, None);
        return None;
    };
    let decision = match hook_decision(&payload, event, title_hint) {
        HookDecisionOutcome::Apply(decision) => decision,
        HookDecisionOutcome::Ignore {
            action,
            display_task_id,
            meta,
        } => {
            append_hook_event_log(
                &payload,
                meta.as_ref(),
                action,
                display_task_id.as_deref(),
                None,
            );
            return None;
        }
    };
    let notification = {
        let mut store = state.store.lock().expect("task store lock poisoned");
        let has_existing_task = store.has_task(&decision.display_task_id);
        if let Some(action) = ignore_action_for_decision(&decision, has_existing_task) {
            append_hook_event_log(
                &payload,
                decision.meta.as_ref(),
                action,
                Some(&decision.display_task_id),
                decision.title_hint.as_deref(),
            );
            return None;
        }

        store.apply_event_with_title(decision.event, decision.title_hint.clone())
    };
    let settings = state
        .settings
        .lock()
        .expect("settings lock poisoned")
        .clone();

    append_hook_event_log(
        &payload,
        decision.meta.as_ref(),
        decision.action,
        Some(&decision.display_task_id),
        decision
            .title_hint
            .as_deref()
            .or(notification.as_ref().map(|notice| notice.title.as_str())),
    );

    notification.map(|notification| (notification, settings))
}

fn hook_decision(
    payload: &CodexHookPayload,
    event: NormalizedEvent,
    explicit_title_hint: Option<String>,
) -> HookDecisionOutcome {
    if matches!(event, NormalizedEvent::PermissionRequested { .. })
        && is_bypassed_permission_mode(&payload.permission_mode)
    {
        return HookDecisionOutcome::Ignore {
            action: HookAction::IgnoredBypassedPermission,
            display_task_id: Some(event_session_id(&event).to_string()),
            meta: payload_session_meta(payload),
        };
    }

    if !payload.agent_id.trim().is_empty()
        && matches!(
            event,
            NormalizedEvent::UserPromptSubmit { .. } | NormalizedEvent::Completed { .. }
        )
    {
        return HookDecisionOutcome::Ignore {
            action: HookAction::IgnoredSubagentLifecycle,
            display_task_id: Some(event_session_id(&event).to_string()),
            meta: None,
        };
    }

    let meta = payload_session_meta(payload);

    if meta
        .as_ref()
        .is_some_and(|meta| meta.thread_source == CodexThreadSource::Subagent)
    {
        return subagent_decision(event, explicit_title_hint, meta);
    }

    let display_task_id = event_session_id(&event).to_string();
    let title_hint = explicit_title_hint.or_else(|| thread_name_for_session(&display_task_id));

    HookDecisionOutcome::Apply(HookDecision {
        event,
        title_hint,
        action: HookAction::Shown,
        display_task_id,
        meta,
    })
}

fn payload_session_meta(payload: &CodexHookPayload) -> Option<CodexSessionMeta> {
    payload
        .agent_transcript_path
        .as_deref()
        .and_then(session_meta_from_transcript)
        .or_else(|| {
            payload
                .transcript_path
                .as_deref()
                .and_then(session_meta_from_transcript)
        })
        .or_else(|| session_meta_for_session(&payload.session_id))
}

fn subagent_decision(
    event: NormalizedEvent,
    explicit_title_hint: Option<String>,
    meta: Option<CodexSessionMeta>,
) -> HookDecisionOutcome {
    let parent_thread_id = meta.as_ref().and_then(|meta| meta.parent_thread_id.clone());
    let Some(parent_thread_id) = parent_thread_id else {
        return HookDecisionOutcome::Ignore {
            action: HookAction::IgnoredSubagentWithoutParent,
            display_task_id: None,
            meta,
        };
    };

    if matches!(
        event,
        NormalizedEvent::UserPromptSubmit { .. } | NormalizedEvent::Completed { .. }
    ) {
        return HookDecisionOutcome::Ignore {
            action: HookAction::IgnoredSubagentLifecycle,
            display_task_id: Some(parent_thread_id),
            meta,
        };
    }

    let title_hint = thread_name_for_session(&parent_thread_id).or(explicit_title_hint);
    HookDecisionOutcome::Apply(HookDecision {
        event: retarget_event(event, parent_thread_id.clone()),
        title_hint,
        action: HookAction::FoldedToParent,
        display_task_id: parent_thread_id,
        meta,
    })
}

fn retarget_event(event: NormalizedEvent, session_id: String) -> NormalizedEvent {
    match event {
        NormalizedEvent::UserPromptSubmit { prompt, .. } => {
            NormalizedEvent::UserPromptSubmit { session_id, prompt }
        }
        NormalizedEvent::PermissionRequested { .. } => {
            NormalizedEvent::PermissionRequested { session_id }
        }
        NormalizedEvent::ConfirmationRequested { .. } => {
            NormalizedEvent::ConfirmationRequested { session_id }
        }
        NormalizedEvent::Completed { .. } => NormalizedEvent::Completed { session_id },
        NormalizedEvent::Interrupted { reason, .. } => {
            NormalizedEvent::Interrupted { session_id, reason }
        }
    }
}

fn event_session_id(event: &NormalizedEvent) -> &str {
    match event {
        NormalizedEvent::UserPromptSubmit { session_id, .. }
        | NormalizedEvent::PermissionRequested { session_id }
        | NormalizedEvent::ConfirmationRequested { session_id }
        | NormalizedEvent::Completed { session_id }
        | NormalizedEvent::Interrupted { session_id, .. } => session_id,
    }
}

fn ignore_action_for_decision(
    decision: &HookDecision,
    has_existing_task: bool,
) -> Option<HookAction> {
    let has_trusted_identity =
        has_existing_task || decision.meta.is_some() || has_meaningful_title(&decision.title_hint);

    match &decision.event {
        NormalizedEvent::UserPromptSubmit { prompt, .. }
            if !has_trusted_identity && is_low_signal_prompt(prompt) =>
        {
            Some(HookAction::IgnoredLowSignalPrompt)
        }
        NormalizedEvent::PermissionRequested { .. }
        | NormalizedEvent::ConfirmationRequested { .. }
        | NormalizedEvent::Completed { .. }
        | NormalizedEvent::Interrupted { .. }
            if !has_trusted_identity =>
        {
            Some(HookAction::IgnoredUntrustedTask)
        }
        _ => None,
    }
}

fn has_meaningful_title(title: &Option<String>) -> bool {
    title
        .as_deref()
        .map(str::trim)
        .is_some_and(|title| !title.is_empty() && title != "Codex task")
}

fn is_low_signal_prompt(prompt: &str) -> bool {
    let trimmed = prompt.trim();
    if trimmed.is_empty() {
        return true;
    }

    if derive_title(trimmed) == "Codex task" {
        return true;
    }

    let normalized = trimmed.to_ascii_lowercase();
    if normalized.starts_with("you are an expert") || normalized.starts_with("you are codex") {
        return true;
    }

    let heading = normalized.trim_start_matches('#').trim();
    heading != normalized
        && matches!(
            heading,
            "overview" | "agents.md" | "agents" | "instructions" | "system prompt"
        )
}

fn is_bypassed_permission_mode(permission_mode: &str) -> bool {
    let normalized = permission_mode
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase();

    normalized.contains("bypass")
        || normalized == "dangerfullaccess"
        || normalized == "fullauto"
}

#[cfg_attr(test, allow(dead_code))]
fn payload_event_name(payload: &CodexHookPayload) -> String {
    if payload.hook_event_name.trim().is_empty() {
        payload.event.trim().to_string()
    } else {
        payload.hook_event_name.trim().to_string()
    }
}

#[cfg(not(test))]
fn append_hook_event_log(
    payload: &CodexHookPayload,
    meta: Option<&CodexSessionMeta>,
    action: HookAction,
    display_task_id: Option<&str>,
    title: Option<&str>,
) {
    let Some(home) = dirs::home_dir() else {
        return;
    };
    let dir = home.join(".codex-status-light");
    if fs::create_dir_all(&dir).is_err() {
        return;
    }

    let timestamp_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    let entry = HookEventLogEntry {
        timestamp_ms,
        event: payload_event_name(payload),
        session_id: payload.session_id.trim().to_string(),
        display_task_id: display_task_id.map(ToString::to_string),
        thread_source: meta.map(|meta| thread_source_label(&meta.thread_source).to_string()),
        parent_thread_id: meta.and_then(|meta| meta.parent_thread_id.clone()),
        action: action.as_str(),
        title: title.map(ToString::to_string),
        cwd: optional_log_value(&payload.cwd),
        permission_mode: optional_log_value(&payload.permission_mode),
        tool_name: optional_log_value(&payload.tool_name),
        agent_id: optional_log_value(&payload.agent_id),
        agent_type: optional_log_value(&payload.agent_type),
        transcript_path_present: payload.transcript_path.is_some(),
        agent_transcript_path_present: payload.agent_transcript_path.is_some(),
    };
    let Ok(line) = serde_json::to_string(&entry) else {
        return;
    };
    let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("events.jsonl"))
    else {
        return;
    };
    let _ = writeln!(file, "{line}");
}

#[cfg(test)]
fn append_hook_event_log(
    _payload: &CodexHookPayload,
    _meta: Option<&CodexSessionMeta>,
    _action: HookAction,
    _display_task_id: Option<&str>,
    _title: Option<&str>,
) {
}

#[cfg(not(test))]
#[derive(serde::Serialize)]
struct HookEventLogEntry {
    timestamp_ms: u128,
    event: String,
    session_id: String,
    display_task_id: Option<String>,
    thread_source: Option<String>,
    parent_thread_id: Option<String>,
    action: &'static str,
    title: Option<String>,
    cwd: Option<String>,
    permission_mode: Option<String>,
    tool_name: Option<String>,
    agent_id: Option<String>,
    agent_type: Option<String>,
    transcript_path_present: bool,
    agent_transcript_path_present: bool,
}

#[cfg(not(test))]
fn thread_source_label(source: &CodexThreadSource) -> &str {
    match source {
        CodexThreadSource::User => "user",
        CodexThreadSource::Subagent => "subagent",
        CodexThreadSource::Unknown(source) => source.as_str(),
    }
}

#[cfg(not(test))]
fn optional_log_value(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

pub async fn handle_codex_hook(
    State(state): State<SharedBackendState>,
    Json(payload): Json<CodexHookPayload>,
) -> StatusCode {
    if let Some((notification, settings)) = notification_for_payload(&state, payload) {
        tokio::spawn(async move {
            let endpoint_url = settings.bark_endpoint_url.clone();
            maybe_send_bark_notification(notification, settings, |outbound| async move {
                BarkNotifier::new(endpoint_url).send(&outbound).await
            })
            .await;
        });
    }
    StatusCode::OK
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    async fn post_payload(state: SharedBackendState, payload: impl Into<Body>) -> StatusCode {
        let response = router(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/codex-hook")
                    .header("content-type", "application/json")
                    .body(payload.into())
                    .unwrap(),
            )
            .await
            .unwrap();

        response.status()
    }

    #[tokio::test]
    async fn accepts_valid_hook_payload_and_creates_visible_task() {
        let state = SharedBackendState::new(TaskStore::new(), AppSettings::default());

        let status = post_payload(
            state.clone(),
            r#"{"event":"UserPromptSubmit","session_id":"s1","prompt":"修复登录页"}"#,
        )
        .await;

        let tasks = state.store.lock().unwrap().visible_tasks();
        assert_eq!(status, StatusCode::OK);
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, "s1");
        assert_eq!(tasks[0].title, "修复登录页");
    }

    #[tokio::test]
    async fn payload_without_session_returns_ok_without_creating_task() {
        let state = SharedBackendState::new(TaskStore::new(), AppSettings::default());

        let status = post_payload(
            state.clone(),
            r#"{"event":"UserPromptSubmit","prompt":"修复登录页"}"#,
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert!(state.store.lock().unwrap().visible_tasks().is_empty());
    }

    #[tokio::test]
    async fn unknown_event_returns_ok_without_creating_task() {
        let state = SharedBackendState::new(TaskStore::new(), AppSettings::default());

        let status = post_payload(
            state.clone(),
            r#"{"event":"SomethingElse","session_id":"s1","prompt":"修复登录页"}"#,
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert!(state.store.lock().unwrap().visible_tasks().is_empty());
    }

    #[tokio::test]
    async fn unknown_permission_request_does_not_create_standalone_task() {
        let state = SharedBackendState::new(TaskStore::new(), AppSettings::default());

        let notification = notification_for_payload(
            &state,
            serde_json::from_str(
                r#"{
                    "event":"PermissionRequest",
                    "session_id":"unknown-permission",
                    "permission_mode":"default",
                    "tool_name":"Bash"
                }"#,
            )
            .unwrap(),
        );

        assert!(notification.is_none());
        assert!(state.store.lock().unwrap().visible_tasks().is_empty());
    }

    #[tokio::test]
    async fn unknown_stop_does_not_create_standalone_completed_task() {
        let state = SharedBackendState::new(TaskStore::new(), AppSettings::default());

        let notification = notification_for_payload(
            &state,
            serde_json::from_str(
                r#"{
                    "event":"Stop",
                    "session_id":"unknown-stop",
                    "last_assistant_message":"Done."
                }"#,
            )
            .unwrap(),
        );

        assert!(notification.is_none());
        assert!(state.store.lock().unwrap().visible_tasks().is_empty());
    }

    #[tokio::test]
    async fn low_signal_unknown_prompts_do_not_create_tasks() {
        let state = SharedBackendState::new(TaskStore::new(), AppSettings::default());

        for (idx, prompt) in [
            "",
            "请帮我",
            "# Overview",
            "You are an expert implementation agent.",
        ]
        .into_iter()
        .enumerate()
        {
            let payload = serde_json::json!({
                "event": "UserPromptSubmit",
                "session_id": format!("unknown-prompt-{idx}"),
                "prompt": prompt,
            });

            assert!(
                notification_for_payload(
                    &state,
                    serde_json::from_value(payload).expect("test payload")
                )
                .is_none()
            );
        }

        assert!(state.store.lock().unwrap().visible_tasks().is_empty());
    }

    #[tokio::test]
    async fn permission_request_for_existing_task_still_updates_status() {
        let state = SharedBackendState::new(TaskStore::new(), AppSettings::default());
        notification_for_payload(
            &state,
            serde_json::from_str(
                r#"{"event":"UserPromptSubmit","session_id":"s1","prompt":"修复登录页"}"#,
            )
            .unwrap(),
        );

        let notification = notification_for_payload(
            &state,
            serde_json::from_str(
                r#"{
                    "event":"PermissionRequest",
                    "session_id":"s1",
                    "permission_mode":"default",
                    "tool_name":"Bash"
                }"#,
            )
            .unwrap(),
        );

        let tasks = state.store.lock().unwrap().visible_tasks();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].status, crate::domain::TaskStatus::NeedsPermission);
        assert!(notification.is_some());
    }

    #[tokio::test]
    async fn bypassed_permission_request_for_existing_task_is_ignored() {
        let state = SharedBackendState::new(TaskStore::new(), AppSettings::default());
        notification_for_payload(
            &state,
            serde_json::from_str(
                r#"{"event":"UserPromptSubmit","session_id":"s1","prompt":"修复登录页"}"#,
            )
            .unwrap(),
        );

        let notification = notification_for_payload(
            &state,
            serde_json::from_str(
                r#"{
                    "event":"PermissionRequest",
                    "session_id":"s1",
                    "permission_mode":"bypassPermissions",
                    "tool_name":"mcp__codex_apps__vercel__list_teams"
                }"#,
            )
            .unwrap(),
        );

        let tasks = state.store.lock().unwrap().visible_tasks();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].status, crate::domain::TaskStatus::Executing);
        assert!(notification.is_none());
    }

    #[tokio::test]
    async fn does_not_attempt_bark_when_notifications_are_disabled_or_endpoint_is_empty() {
        let notification = crate::task_store::NotificationRequest {
            task_id: "s1".into(),
            status: crate::domain::TaskStatus::NeedsPermission,
            title: "修复登录页".into(),
        };
        let mut attempts = 0;

        maybe_send_bark_notification(
            notification.clone(),
            AppSettings {
                notifications_enabled: false,
                bark_endpoint_url: "https://api.day.app/key".into(),
                ..AppSettings::default()
            },
            |_| {
                attempts += 1;
                async { Ok(()) }
            },
        )
        .await;
        maybe_send_bark_notification(
            notification,
            AppSettings {
                notifications_enabled: true,
                bark_endpoint_url: "  ".into(),
                ..AppSettings::default()
            },
            |_| {
                attempts += 1;
                async { Ok(()) }
            },
        )
        .await;

        assert_eq!(attempts, 0);
    }

    #[tokio::test]
    async fn attempts_bark_when_enabled_endpoint_and_notification_exist() {
        let state = SharedBackendState::new(
            TaskStore::new(),
            AppSettings {
                notifications_enabled: true,
                bark_endpoint_url: "https://api.day.app/key".into(),
                ..AppSettings::default()
            },
        );
        let mut attempts = 0;
        notification_for_payload(
            &state,
            serde_json::from_str(
                r#"{"event":"UserPromptSubmit","session_id":"s1","prompt":"修复登录页"}"#,
            )
            .unwrap(),
        );
        let (notification, settings) = notification_for_payload(
            &state,
            serde_json::from_str(r#"{"event":"PermissionRequest","session_id":"s1"}"#).unwrap(),
        )
        .unwrap();

        maybe_send_bark_notification(notification, settings, |outbound| {
            attempts += 1;
            assert_eq!(outbound.title, "修复登录页");
            async { Ok(()) }
        })
        .await;

        assert_eq!(attempts, 1);
    }

    #[tokio::test]
    async fn uses_thread_name_as_notification_title_when_available() {
        let state = SharedBackendState::new(
            TaskStore::new(),
            AppSettings {
                notifications_enabled: true,
                bark_endpoint_url: "https://api.day.app/key".into(),
                ..AppSettings::default()
            },
        );
        notification_for_payload_with_title(
            &state,
            serde_json::from_str(
                r#"{"event":"UserPromptSubmit","session_id":"s1","prompt":"如果现在10个同步为通过"}"#,
            )
            .unwrap(),
            Some("调研红绿黄灯项目".into()),
        );
        let (notification, settings) = notification_for_payload_with_title(
            &state,
            serde_json::from_str(r#"{"event":"PermissionRequest","session_id":"s1"}"#).unwrap(),
            Some("调研红绿黄灯项目".into()),
        )
        .unwrap();
        let mut attempts = 0;

        maybe_send_bark_notification(notification, settings, |outbound| {
            attempts += 1;
            assert_eq!(outbound.title, "调研红绿黄灯项目");
            async { Ok(()) }
        })
        .await;

        assert_eq!(attempts, 1);
    }

    #[tokio::test]
    async fn does_not_attempt_bark_when_task_starts() {
        let state = SharedBackendState::new(
            TaskStore::new(),
            AppSettings {
                notifications_enabled: true,
                bark_endpoint_url: "https://api.day.app/key".into(),
                ..AppSettings::default()
            },
        );
        let notification = notification_for_payload(
            &state,
            serde_json::from_str(
                r#"{"event":"UserPromptSubmit","session_id":"s1","prompt":"修复登录页"}"#,
            )
            .unwrap(),
        );

        assert!(notification.is_none());
        let tasks = state.store.lock().unwrap().visible_tasks();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].status, crate::domain::TaskStatus::Executing);
    }

    #[tokio::test]
    async fn bark_failure_does_not_change_hook_response() {
        let state = SharedBackendState::new(
            TaskStore::new(),
            AppSettings {
                notifications_enabled: true,
                bark_endpoint_url: "http://127.0.0.1:1".into(),
                ..AppSettings::default()
            },
        );
        post_payload(
            state.clone(),
            r#"{"event":"UserPromptSubmit","session_id":"s1","prompt":"修复登录页"}"#,
        )
        .await;

        let status =
            post_payload(state, r#"{"event":"PermissionRequest","session_id":"s1"}"#).await;

        assert_eq!(status, StatusCode::OK);
    }

    #[tokio::test]
    async fn subagent_permission_folds_into_parent_task() {
        let transcript_dir = tempfile::tempdir().unwrap();
        let transcript_path = transcript_dir.path().join("subagent.jsonl");
        std::fs::write(
            &transcript_path,
            r#"{"timestamp":"2026-05-29T09:23:50.505Z","type":"session_meta","payload":{"id":"child-1","cwd":"/tmp/project","source":{"subagent":{"thread_spawn":{"parent_thread_id":"parent-1","depth":1,"agent_nickname":"Banach","agent_role":"worker"}}},"thread_source":"subagent"}}"#,
        )
        .unwrap();
        let state = SharedBackendState::new(
            TaskStore::new(),
            AppSettings {
                notifications_enabled: true,
                bark_endpoint_url: "https://api.day.app/key".into(),
                ..AppSettings::default()
            },
        );
        notification_for_payload_with_title(
            &state,
            serde_json::from_str(
                r#"{"event":"UserPromptSubmit","session_id":"parent-1","prompt":"调研红绿黄灯项目"}"#,
            )
            .unwrap(),
            Some("调研红绿黄灯项目".into()),
        );

        let payload = format!(
            r#"{{"event":"PermissionRequest","session_id":"child-1","transcript_path":{}}}"#,
            serde_json::to_string(transcript_path.to_str().unwrap()).unwrap()
        );
        let (notification, _settings) =
            notification_for_payload(&state, serde_json::from_str(&payload).unwrap()).unwrap();

        let tasks = state.store.lock().unwrap().visible_tasks();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, "parent-1");
        assert_eq!(tasks[0].title, "调研红绿黄灯项目");
        assert_eq!(tasks[0].status, crate::domain::TaskStatus::NeedsPermission);
        assert_eq!(notification.task_id, "parent-1");
        assert_eq!(notification.title, "调研红绿黄灯项目");
    }

    #[tokio::test]
    async fn subagent_start_and_stop_do_not_create_standalone_rows() {
        let transcript_dir = tempfile::tempdir().unwrap();
        let transcript_path = transcript_dir.path().join("subagent.jsonl");
        std::fs::write(
            &transcript_path,
            r#"{"timestamp":"2026-05-29T09:23:50.505Z","type":"session_meta","payload":{"id":"child-1","cwd":"/tmp/project","source":{"subagent":{"thread_spawn":{"parent_thread_id":"parent-1","depth":1,"agent_nickname":"Banach","agent_role":"worker"}}},"thread_source":"subagent"}}"#,
        )
        .unwrap();
        let state = SharedBackendState::new(TaskStore::new(), AppSettings::default());
        notification_for_payload_with_title(
            &state,
            serde_json::from_str(
                r#"{"event":"UserPromptSubmit","session_id":"parent-1","prompt":"调研红绿黄灯项目"}"#,
            )
            .unwrap(),
            Some("调研红绿黄灯项目".into()),
        );

        for event in ["UserPromptSubmit", "Stop"] {
            let payload = format!(
                r##"{{"event":"{}","session_id":"child-1","prompt":"# Overview","transcript_path":{}}}"##,
                event,
                serde_json::to_string(transcript_path.to_str().unwrap()).unwrap()
            );
            assert!(
                notification_for_payload(&state, serde_json::from_str(&payload).unwrap()).is_none()
            );
        }

        let tasks = state.store.lock().unwrap().visible_tasks();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, "parent-1");
        assert_eq!(tasks[0].title, "调研红绿黄灯项目");
        assert_eq!(tasks[0].status, crate::domain::TaskStatus::Executing);
    }
}
