use std::future::Future;
use std::sync::{Arc, Mutex};

use axum::{extract::State, http::StatusCode, routing::post, Json, Router};

use crate::codex_provider::{normalize_payload, CodexHookPayload};
use crate::notifier::{BarkNotifier, OutboundNotification};
use crate::settings::AppSettings;
use crate::task_store::{NotificationRequest, TaskStore};

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
    let event = normalize_payload(payload)?;
    let notification = {
        let mut store = state.store.lock().expect("task store lock poisoned");
        store.apply_event(event)
    }?;
    let settings = state
        .settings
        .lock()
        .expect("settings lock poisoned")
        .clone();

    Some((notification, settings))
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

    async fn post_payload(state: SharedBackendState, payload: &'static str) -> StatusCode {
        let response = router(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/codex-hook")
                    .header("content-type", "application/json")
                    .body(Body::from(payload))
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
}
