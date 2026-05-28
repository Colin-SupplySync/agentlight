use std::sync::{Arc, Mutex};

use axum::{extract::State, http::StatusCode, routing::post, Json, Router};

use crate::codex_provider::{normalize_payload, CodexHookPayload};
use crate::notifier::{BarkNotifier, OutboundNotification};
use crate::settings::AppSettings;
use crate::task_store::TaskStore;

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

pub async fn handle_codex_hook(
    State(state): State<SharedBackendState>,
    Json(payload): Json<CodexHookPayload>,
) -> StatusCode {
    let Some(event) = normalize_payload(payload) else {
        return StatusCode::OK;
    };

    let notification = {
        let mut store = state.store.lock().expect("task store lock poisoned");
        store.apply_event(event)
    };

    if let Some(notification) = notification {
        let settings = state
            .settings
            .lock()
            .expect("settings lock poisoned")
            .clone();
        if settings.notifications_enabled && !settings.bark_endpoint_url.trim().is_empty() {
            let notifier = BarkNotifier::new(settings.bark_endpoint_url);
            let outbound = OutboundNotification {
                status: notification.status,
                title: notification.title,
            };
            tokio::spawn(async move {
                let _ = notifier.send(&outbound).await;
            });
        }
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
}
