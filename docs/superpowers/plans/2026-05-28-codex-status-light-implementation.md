# Codex Status Light Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the macOS Codex status-light MVP: a compact Tauri desktop overlay, Codex hook ingestion, five-state task engine, and Bark one-line iPhone notifications.

**Architecture:** Use a Tauri v2 app in `app/`. Rust owns provider ingestion, task state, settings, hook installation, and Bark notifications. React/TypeScript renders the overlay and settings from backend DTOs and sends only user actions such as "viewed task" or "save settings."

**Tech Stack:** Tauri v2, Rust, React, TypeScript, Vite, Vitest, React Testing Library, Axum/Tokio for the local hook HTTP endpoint, serde/serde_json for payloads and settings.

---

## Scope Check

The approved spec covers one shippable MVP. It has multiple modules, but they are tightly coupled around one flow: Codex hooks -> normalized events -> task state -> desktop overlay -> Bark notification. Keep this as one implementation plan.

## File Structure

Create the Tauri app under `app/` so the existing `docs/` tree remains at the repository root.

Backend files:

- `app/src-tauri/src/lib.rs` wires Tauri setup, managed state, commands, and hook server startup.
- `app/src-tauri/src/domain.rs` defines task states, normalized events, task records, and reducer behavior.
- `app/src-tauri/src/title.rs` derives short task titles from prompts.
- `app/src-tauri/src/confirmation.rs` detects conservative "needs confirmation" assistant messages.
- `app/src-tauri/src/codex_provider.rs` converts Codex hook payloads into normalized events.
- `app/src-tauri/src/task_store.rs` owns in-memory tasks, viewed/disappearance timers, and notification dedupe decisions.
- `app/src-tauri/src/notifier.rs` defines `Notifier`, `BarkNotifier`, and test fakes.
- `app/src-tauri/src/settings.rs` loads/saves Bark URL, notification toggle, overlay position, and hook status.
- `app/src-tauri/src/hook_server.rs` exposes a local HTTP endpoint for hook scripts.
- `app/src-tauri/src/hook_installer.rs` installs/updates user-level Codex hooks in `~/.codex/hooks.json`.
- `app/src-tauri/src/dto.rs` defines frontend-facing serializable DTOs.

Frontend files:

- `app/src/App.tsx` composes overlay and settings panel.
- `app/src/main.tsx` boots React.
- `app/src/types.ts` mirrors frontend DTOs.
- `app/src/api.ts` wraps Tauri commands and event subscription.
- `app/src/components/TaskOverlay.tsx` renders the floating task stack.
- `app/src/components/SettingsPanel.tsx` renders minimal settings and hook install status.
- `app/src/components/TrafficDots.tsx` renders green/yellow/red dot state.
- `app/src/styles.css` styles the compact overlay.
- `app/src/test/setup.ts` configures frontend tests.

Scripts:

- `app/scripts/codex-status-hook.js` forwards Codex hook JSON from stdin to the local Tauri endpoint.

Docs:

- `app/README.md` documents local development, Bark setup, and hook installation.

## Task 1: Scaffold The Tauri App

**Files:**

- Create: `app/`
- Create: `app/package.json`
- Create: `app/src-tauri/`
- Modify: `.gitignore`

- [ ] **Step 1: Initialize git if needed**

Run from repository root:

```bash
git rev-parse --is-inside-work-tree || git init
```

Expected: either prints `true` or initializes an empty repository.

- [ ] **Step 2: Scaffold Tauri React TypeScript app**

Run from repository root:

```bash
npm create tauri-app@latest app -- --manager npm --template react-ts --identifier com.codexstatus.light --tauri-version 2 --yes
```

Expected: `app/` contains a Tauri v2 React TypeScript project.

- [ ] **Step 3: Install dependencies**

Run:

```bash
cd app
npm install
npm install -D vitest jsdom @testing-library/react @testing-library/jest-dom @testing-library/user-event
npm install @tauri-apps/api
cd src-tauri
cargo add serde --features derive
cargo add serde_json
cargo add tokio --features full
cargo add axum
cargo add tower
cargo add reqwest --features json,rustls-tls
cargo add dirs
cargo add tempfile --dev
cd ../..
```

Expected: frontend and Rust dependencies install successfully.

- [ ] **Step 4: Add project ignores**

Append to `.gitignore`:

```gitignore
.superpowers/
app/node_modules/
app/dist/
app/src-tauri/target/
```

- [ ] **Step 5: Add test scripts**

Modify `app/package.json` scripts to include:

```json
{
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "preview": "vite preview",
    "tauri": "tauri",
    "test": "vitest run",
    "test:watch": "vitest"
  }
}
```

Keep any Tauri scaffold scripts that already exist if they are equivalent.

- [ ] **Step 6: Verify scaffold runs**

Run:

```bash
cd app
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
```

Expected: frontend build succeeds and Rust tests pass.

- [ ] **Step 7: Commit**

Run:

```bash
git add .gitignore app package-lock.json docs/superpowers
git commit -m "chore: scaffold codex status light app"
```

Expected: one scaffold commit.

## Task 2: Implement Core Domain State

**Files:**

- Create: `app/src-tauri/src/domain.rs`
- Create: `app/src-tauri/src/title.rs`
- Modify: `app/src-tauri/src/lib.rs`

- [ ] **Step 1: Write failing domain tests**

Create `app/src-tauri/src/domain.rs` with tests first:

```rust
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Executing,
    NeedsPermission,
    NeedsConfirmation,
    Completed,
    Interrupted,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NormalizedEvent {
    UserPromptSubmit { session_id: String, prompt: String },
    PermissionRequested { session_id: String },
    ConfirmationRequested { session_id: String },
    Completed { session_id: String },
    Interrupted { session_id: String, reason: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskRecord {
    pub id: String,
    pub title: String,
    pub provider: String,
    pub status: TaskStatus,
    pub viewed: bool,
}

pub fn reduce_task(existing: Option<TaskRecord>, event: NormalizedEvent) -> TaskRecord {
    let _ = existing;
    let _ = event;
    unimplemented!("implemented in Step 3")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_creates_executing_task() {
        let task = reduce_task(None, NormalizedEvent::UserPromptSubmit {
            session_id: "s1".into(),
            prompt: "帮我修复移动端导航遮挡并跑测试".into(),
        });

        assert_eq!(task.id, "s1");
        assert_eq!(task.title, "修复移动端导航遮挡");
        assert_eq!(task.provider, "Codex");
        assert_eq!(task.status, TaskStatus::Executing);
        assert!(!task.viewed);
    }

    #[test]
    fn permission_keeps_existing_title_and_sets_yellow_state() {
        let existing = TaskRecord {
            id: "s1".into(),
            title: "修复移动端导航遮挡".into(),
            provider: "Codex".into(),
            status: TaskStatus::Executing,
            viewed: false,
        };

        let task = reduce_task(Some(existing), NormalizedEvent::PermissionRequested {
            session_id: "s1".into(),
        });

        assert_eq!(task.title, "修复移动端导航遮挡");
        assert_eq!(task.status, TaskStatus::NeedsPermission);
    }

    #[test]
    fn completed_sets_red_state() {
        let task = reduce_task(None, NormalizedEvent::Completed {
            session_id: "s2".into(),
        });

        assert_eq!(task.title, "Codex task");
        assert_eq!(task.status, TaskStatus::Completed);
    }

    #[test]
    fn interrupted_sets_flashing_state() {
        let task = reduce_task(None, NormalizedEvent::Interrupted {
            session_id: "s3".into(),
            reason: "connection lost".into(),
        });

        assert_eq!(task.status, TaskStatus::Interrupted);
    }
}
```

- [ ] **Step 2: Run tests to verify failure**

Run:

```bash
cargo test --manifest-path app/src-tauri/Cargo.toml domain::tests -- --nocapture
```

Expected: tests fail because `reduce_task` is not implemented.

- [ ] **Step 3: Add title derivation**

Create `app/src-tauri/src/title.rs`:

```rust
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
```

- [ ] **Step 4: Implement reducer**

Replace `reduce_task` in `domain.rs`:

```rust
use crate::title::derive_title;

pub fn reduce_task(existing: Option<TaskRecord>, event: NormalizedEvent) -> TaskRecord {
    match event {
        NormalizedEvent::UserPromptSubmit { session_id, prompt } => TaskRecord {
            id: session_id,
            title: derive_title(&prompt),
            provider: "Codex".to_string(),
            status: TaskStatus::Executing,
            viewed: false,
        },
        NormalizedEvent::PermissionRequested { session_id } => transition(
            existing,
            session_id,
            TaskStatus::NeedsPermission,
        ),
        NormalizedEvent::ConfirmationRequested { session_id } => transition(
            existing,
            session_id,
            TaskStatus::NeedsConfirmation,
        ),
        NormalizedEvent::Completed { session_id } => transition(
            existing,
            session_id,
            TaskStatus::Completed,
        ),
        NormalizedEvent::Interrupted { session_id, reason: _ } => transition(
            existing,
            session_id,
            TaskStatus::Interrupted,
        ),
    }
}

fn transition(existing: Option<TaskRecord>, session_id: String, status: TaskStatus) -> TaskRecord {
    if let Some(mut task) = existing {
        task.status = status;
        task.viewed = false;
        return task;
    }

    TaskRecord {
        id: session_id,
        title: "Codex task".to_string(),
        provider: "Codex".to_string(),
        status,
        viewed: false,
    }
}
```

Add modules to `app/src-tauri/src/lib.rs`:

```rust
mod domain;
mod title;
```

- [ ] **Step 5: Run tests**

Run:

```bash
cargo test --manifest-path app/src-tauri/Cargo.toml domain::tests title::tests
```

Expected: all domain and title tests pass.

- [ ] **Step 6: Commit**

Run:

```bash
git add app/src-tauri/src/domain.rs app/src-tauri/src/title.rs app/src-tauri/src/lib.rs
git commit -m "feat: add task domain state"
```

## Task 3: Add Confirmation Detection And Codex Payload Normalization

**Files:**

- Create: `app/src-tauri/src/confirmation.rs`
- Create: `app/src-tauri/src/codex_provider.rs`
- Modify: `app/src-tauri/src/lib.rs`

- [ ] **Step 1: Write confirmation detector tests**

Create `app/src-tauri/src/confirmation.rs`:

```rust
pub fn needs_confirmation(message: &str) -> bool {
    let _ = message;
    unimplemented!("implemented in Step 3")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_chinese_confirmation_phrases() {
        assert!(needs_confirmation("请选择下一步要执行的方案。"));
        assert!(needs_confirmation("是否继续执行这些修改？"));
        assert!(needs_confirmation("你希望我采用哪一种方式？"));
    }

    #[test]
    fn detects_english_confirmation_phrases() {
        assert!(needs_confirmation("Please confirm whether I should proceed."));
        assert!(needs_confirmation("Approve this plan before I continue."));
    }

    #[test]
    fn ordinary_completion_is_not_confirmation() {
        assert!(!needs_confirmation("已完成修改并通过测试。"));
    }
}
```

- [ ] **Step 2: Run tests to verify failure**

Run:

```bash
cargo test --manifest-path app/src-tauri/Cargo.toml confirmation::tests
```

Expected: tests fail because detector is not implemented.

- [ ] **Step 3: Implement confirmation detector**

Replace `needs_confirmation`:

```rust
pub fn needs_confirmation(message: &str) -> bool {
    let lower = message.to_lowercase();
    let phrases = [
        "请选择",
        "是否继续",
        "需要确认",
        "你希望",
        "要不要",
        "confirm",
        "approve",
        "proceed",
    ];

    phrases.iter().any(|phrase| lower.contains(phrase))
}
```

- [ ] **Step 4: Write Codex provider tests**

Create `app/src-tauri/src/codex_provider.rs`:

```rust
use crate::confirmation::needs_confirmation;
use crate::domain::NormalizedEvent;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct CodexHookPayload {
    #[serde(default)]
    pub event: String,
    #[serde(default)]
    pub session_id: String,
    #[serde(default)]
    pub prompt: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
}

pub fn normalize_payload(payload: CodexHookPayload) -> Option<NormalizedEvent> {
    let _ = payload;
    unimplemented!("implemented in Step 6")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_prompt_maps_to_executing_event() {
        let event = normalize_payload(CodexHookPayload {
            event: "UserPromptSubmit".into(),
            session_id: "s1".into(),
            prompt: Some("帮我修复导航".into()),
            message: None,
            error: None,
        });

        assert_eq!(
            event,
            Some(NormalizedEvent::UserPromptSubmit {
                session_id: "s1".into(),
                prompt: "帮我修复导航".into()
            })
        );
    }

    #[test]
    fn permission_maps_to_needs_permission() {
        let event = normalize_payload(CodexHookPayload {
            event: "PermissionRequest".into(),
            session_id: "s1".into(),
            prompt: None,
            message: None,
            error: None,
        });

        assert_eq!(event, Some(NormalizedEvent::PermissionRequested { session_id: "s1".into() }));
    }

    #[test]
    fn stop_with_confirmation_phrase_maps_to_needs_confirmation() {
        let event = normalize_payload(CodexHookPayload {
            event: "Stop".into(),
            session_id: "s1".into(),
            prompt: None,
            message: Some("是否继续执行下一步？".into()),
            error: None,
        });

        assert_eq!(event, Some(NormalizedEvent::ConfirmationRequested { session_id: "s1".into() }));
    }

    #[test]
    fn stop_without_confirmation_maps_to_completed() {
        let event = normalize_payload(CodexHookPayload {
            event: "Stop".into(),
            session_id: "s1".into(),
            prompt: None,
            message: Some("已完成。".into()),
            error: None,
        });

        assert_eq!(event, Some(NormalizedEvent::Completed { session_id: "s1".into() }));
    }

    #[test]
    fn empty_session_is_ignored() {
        let event = normalize_payload(CodexHookPayload {
            event: "Stop".into(),
            session_id: "".into(),
            prompt: None,
            message: None,
            error: None,
        });

        assert_eq!(event, None);
    }
}
```

- [ ] **Step 5: Run provider tests to verify failure**

Run:

```bash
cargo test --manifest-path app/src-tauri/Cargo.toml codex_provider::tests
```

Expected: tests fail because `normalize_payload` is not implemented.

- [ ] **Step 6: Implement payload normalization**

Replace `normalize_payload`:

```rust
pub fn normalize_payload(payload: CodexHookPayload) -> Option<NormalizedEvent> {
    let session_id = payload.session_id.trim().to_string();
    if session_id.is_empty() {
        return None;
    }

    match payload.event.as_str() {
        "UserPromptSubmit" => Some(NormalizedEvent::UserPromptSubmit {
            session_id,
            prompt: payload.prompt.unwrap_or_default(),
        }),
        "PermissionRequest" => Some(NormalizedEvent::PermissionRequested { session_id }),
        "Stop" => {
            let message = payload.message.unwrap_or_default();
            if needs_confirmation(&message) {
                Some(NormalizedEvent::ConfirmationRequested { session_id })
            } else {
                Some(NormalizedEvent::Completed { session_id })
            }
        }
        "Error" | "ToolFailure" | "ConnectionLost" => Some(NormalizedEvent::Interrupted {
            session_id,
            reason: payload.error.unwrap_or_else(|| payload.event),
        }),
        _ => None,
    }
}
```

Add modules to `lib.rs`:

```rust
mod codex_provider;
mod confirmation;
```

- [ ] **Step 7: Run tests**

Run:

```bash
cargo test --manifest-path app/src-tauri/Cargo.toml confirmation::tests codex_provider::tests
```

Expected: all confirmation and provider tests pass.

- [ ] **Step 8: Commit**

Run:

```bash
git add app/src-tauri/src/confirmation.rs app/src-tauri/src/codex_provider.rs app/src-tauri/src/lib.rs
git commit -m "feat: normalize codex hook events"
```

## Task 4: Implement Task Store And Notification Decisions

**Files:**

- Create: `app/src-tauri/src/task_store.rs`
- Modify: `app/src-tauri/src/lib.rs`

- [ ] **Step 1: Write failing task store tests**

Create `app/src-tauri/src/task_store.rs`:

```rust
use crate::domain::{reduce_task, NormalizedEvent, TaskRecord, TaskStatus};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotificationRequest {
    pub task_id: String,
    pub status: TaskStatus,
    pub title: String,
}

pub struct TaskStore {
    tasks: HashMap<String, TaskRecord>,
    notified: HashSet<(String, TaskStatus)>,
    viewed_at: HashMap<String, Instant>,
}

impl TaskStore {
    pub fn new() -> Self {
        Self {
            tasks: HashMap::new(),
            notified: HashSet::new(),
            viewed_at: HashMap::new(),
        }
    }

    pub fn apply_event(&mut self, event: NormalizedEvent) -> Option<NotificationRequest> {
        let _ = event;
        unimplemented!("implemented in Step 3")
    }

    pub fn mark_viewed(&mut self, task_id: &str, now: Instant) {
        let _ = (task_id, now);
        unimplemented!("implemented in Step 3")
    }

    pub fn remove_expired_viewed(&mut self, now: Instant, delay: Duration) {
        let _ = (now, delay);
        unimplemented!("implemented in Step 3")
    }

    pub fn visible_tasks(&self) -> Vec<TaskRecord> {
        self.tasks.values().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn executing_does_not_notify() {
        let mut store = TaskStore::new();
        let notification = store.apply_event(NormalizedEvent::UserPromptSubmit {
            session_id: "s1".into(),
            prompt: "修复登录页".into(),
        });
        assert_eq!(notification, None);
    }

    #[test]
    fn permission_notifies_once_per_state() {
        let mut store = TaskStore::new();
        store.apply_event(NormalizedEvent::UserPromptSubmit {
            session_id: "s1".into(),
            prompt: "修复登录页".into(),
        });

        let first = store.apply_event(NormalizedEvent::PermissionRequested { session_id: "s1".into() });
        let second = store.apply_event(NormalizedEvent::PermissionRequested { session_id: "s1".into() });

        assert!(first.is_some());
        assert_eq!(second, None);
    }

    #[test]
    fn completed_disappears_after_viewed_delay() {
        let mut store = TaskStore::new();
        let now = Instant::now();
        store.apply_event(NormalizedEvent::Completed { session_id: "s1".into() });
        store.mark_viewed("s1", now);
        store.remove_expired_viewed(now + Duration::from_secs(14), Duration::from_secs(15));
        assert_eq!(store.visible_tasks().len(), 1);

        store.remove_expired_viewed(now + Duration::from_secs(15), Duration::from_secs(15));
        assert!(store.visible_tasks().is_empty());
    }

    #[test]
    fn yellow_states_do_not_disappear_after_viewed() {
        let mut store = TaskStore::new();
        let now = Instant::now();
        store.apply_event(NormalizedEvent::PermissionRequested { session_id: "s1".into() });
        store.mark_viewed("s1", now);
        store.remove_expired_viewed(now + Duration::from_secs(30), Duration::from_secs(15));
        assert_eq!(store.visible_tasks().len(), 1);
    }
}
```

- [ ] **Step 2: Run tests to verify failure**

Run:

```bash
cargo test --manifest-path app/src-tauri/Cargo.toml task_store::tests
```

Expected: tests fail because store methods are not implemented.

- [ ] **Step 3: Implement task store**

Replace the unimplemented methods:

```rust
    pub fn apply_event(&mut self, event: NormalizedEvent) -> Option<NotificationRequest> {
        let task_id = match &event {
            NormalizedEvent::UserPromptSubmit { session_id, .. }
            | NormalizedEvent::PermissionRequested { session_id }
            | NormalizedEvent::ConfirmationRequested { session_id }
            | NormalizedEvent::Completed { session_id }
            | NormalizedEvent::Interrupted { session_id, .. } => session_id.clone(),
        };

        let existing = self.tasks.remove(&task_id);
        let task = reduce_task(existing, event);
        self.viewed_at.remove(&task_id);

        let notification = if is_notifiable(&task.status)
            && self.notified.insert((task.id.clone(), task.status.clone()))
        {
            Some(NotificationRequest {
                task_id: task.id.clone(),
                status: task.status.clone(),
                title: task.title.clone(),
            })
        } else {
            None
        };

        self.tasks.insert(task_id, task);
        notification
    }

    pub fn mark_viewed(&mut self, task_id: &str, now: Instant) {
        if let Some(task) = self.tasks.get_mut(task_id) {
            task.viewed = true;
            self.viewed_at.insert(task_id.to_string(), now);
        }
    }

    pub fn remove_expired_viewed(&mut self, now: Instant, delay: Duration) {
        let expired: Vec<String> = self.viewed_at
            .iter()
            .filter_map(|(task_id, viewed_at)| {
                let task = self.tasks.get(task_id)?;
                let removable = matches!(task.status, TaskStatus::Completed | TaskStatus::Interrupted);
                if removable && now.duration_since(*viewed_at) >= delay {
                    Some(task_id.clone())
                } else {
                    None
                }
            })
            .collect();

        for task_id in expired {
            self.tasks.remove(&task_id);
            self.viewed_at.remove(&task_id);
        }
    }

fn is_notifiable(status: &TaskStatus) -> bool {
    matches!(
        status,
        TaskStatus::NeedsPermission
            | TaskStatus::NeedsConfirmation
            | TaskStatus::Completed
            | TaskStatus::Interrupted
    )
}
```

Add module to `lib.rs`:

```rust
mod task_store;
```

- [ ] **Step 4: Run tests**

Run:

```bash
cargo test --manifest-path app/src-tauri/Cargo.toml task_store::tests
```

Expected: all task store tests pass.

- [ ] **Step 5: Commit**

Run:

```bash
git add app/src-tauri/src/task_store.rs app/src-tauri/src/lib.rs
git commit -m "feat: add task store and notification decisions"
```

## Task 5: Implement Bark Notifier

**Files:**

- Create: `app/src-tauri/src/notifier.rs`
- Modify: `app/src-tauri/src/lib.rs`

- [ ] **Step 1: Write notifier tests**

Create `app/src-tauri/src/notifier.rs`:

```rust
use crate::domain::TaskStatus;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutboundNotification {
    pub status: TaskStatus,
    pub title: String,
}

pub fn notification_text(notification: &OutboundNotification) -> String {
    let _ = notification;
    unimplemented!("implemented in Step 3")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_permission_notification() {
        assert_eq!(
            notification_text(&OutboundNotification {
                status: TaskStatus::NeedsPermission,
                title: "重构登录页权限判断".into(),
            }),
            "需要权限：重构登录页权限判断"
        );
    }

    #[test]
    fn formats_interrupted_notification() {
        assert_eq!(
            notification_text(&OutboundNotification {
                status: TaskStatus::Interrupted,
                title: "接入支付回调测试".into(),
            }),
            "异常中断：接入支付回调测试"
        );
    }
}
```

- [ ] **Step 2: Run tests to verify failure**

Run:

```bash
cargo test --manifest-path app/src-tauri/Cargo.toml notifier::tests
```

Expected: tests fail because formatter is not implemented.

- [ ] **Step 3: Implement formatter and Bark sender**

Replace `notification_text` and add sender:

```rust
#[derive(Clone, Debug)]
pub struct BarkNotifier {
    endpoint_url: String,
    client: reqwest::Client,
}

impl BarkNotifier {
    pub fn new(endpoint_url: String) -> Self {
        Self {
            endpoint_url,
            client: reqwest::Client::new(),
        }
    }

    pub async fn send(&self, notification: &OutboundNotification) -> Result<(), String> {
        let text = notification_text(notification);
        let url = format!("{}/{}", self.endpoint_url.trim_end_matches('/'), text);
        let response = self.client.get(url).send().await.map_err(|err| err.to_string())?;
        if response.status().is_success() {
            Ok(())
        } else {
            Err(format!("Bark returned {}", response.status()))
        }
    }
}

pub fn notification_text(notification: &OutboundNotification) -> String {
    let status = match notification.status {
        TaskStatus::NeedsPermission => "需要权限",
        TaskStatus::NeedsConfirmation => "需要确认",
        TaskStatus::Completed => "已完成",
        TaskStatus::Interrupted => "异常中断",
        TaskStatus::Executing => "执行中",
    };

    format!("{}：{}", status, notification.title)
}
```

Add module to `lib.rs`:

```rust
mod notifier;
```

- [ ] **Step 4: Run tests**

Run:

```bash
cargo test --manifest-path app/src-tauri/Cargo.toml notifier::tests
```

Expected: notifier formatting tests pass.

- [ ] **Step 5: Commit**

Run:

```bash
git add app/src-tauri/src/notifier.rs app/src-tauri/src/lib.rs
git commit -m "feat: add bark notification formatting"
```

## Task 6: Add Settings Persistence And Hook Installer

**Files:**

- Create: `app/src-tauri/src/settings.rs`
- Create: `app/src-tauri/src/hook_installer.rs`
- Create: `app/scripts/codex-status-hook.js`
- Modify: `app/src-tauri/src/lib.rs`

- [ ] **Step 1: Write settings tests**

Create `app/src-tauri/src/settings.rs`:

```rust
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OverlayPosition {
    TopRight,
    TopLeft,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppSettings {
    pub bark_endpoint_url: String,
    pub notifications_enabled: bool,
    pub overlay_position: OverlayPosition,
    pub start_at_login: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            bark_endpoint_url: String::new(),
            notifications_enabled: false,
            overlay_position: OverlayPosition::TopRight,
            start_at_login: false,
        }
    }
}

pub fn load_settings(path: &Path) -> AppSettings {
    let _ = path;
    unimplemented!("implemented in Step 3")
}

pub fn save_settings(path: &Path, settings: &AppSettings) -> Result<(), String> {
    let _ = (path, settings);
    unimplemented!("implemented in Step 3")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_loads_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let settings = load_settings(&dir.path().join("settings.json"));
        assert_eq!(settings, AppSettings::default());
    }

    #[test]
    fn saves_and_loads_settings() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let settings = AppSettings {
            bark_endpoint_url: "https://api.day.app/key".into(),
            notifications_enabled: true,
            overlay_position: OverlayPosition::TopLeft,
            start_at_login: true,
        };

        save_settings(&path, &settings).unwrap();
        assert_eq!(load_settings(&path), settings);
    }
}
```

- [ ] **Step 2: Run settings tests to verify failure**

Run:

```bash
cargo test --manifest-path app/src-tauri/Cargo.toml settings::tests
```

Expected: tests fail because load/save are not implemented.

- [ ] **Step 3: Implement settings load/save**

Replace functions:

```rust
pub fn load_settings(path: &Path) -> AppSettings {
    let Ok(contents) = fs::read_to_string(path) else {
        return AppSettings::default();
    };

    serde_json::from_str(&contents).unwrap_or_default()
}

pub fn save_settings(path: &Path, settings: &AppSettings) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }

    let contents = serde_json::to_string_pretty(settings).map_err(|err| err.to_string())?;
    fs::write(path, contents).map_err(|err| err.to_string())
}
```

- [ ] **Step 4: Write hook installer tests**

Create `app/src-tauri/src/hook_installer.rs`:

```rust
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

pub fn install_hooks(hooks_path: &Path, hook_script_path: &Path) -> Result<(), String> {
    let _ = (hooks_path, hook_script_path);
    unimplemented!("implemented in Step 6")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_hooks_json_with_codex_status_hooks() {
        let dir = tempfile::tempdir().unwrap();
        let hooks_path = dir.path().join("hooks.json");
        let script_path = dir.path().join("codex-status-hook.js");

        install_hooks(&hooks_path, &script_path).unwrap();
        let json: Value = serde_json::from_str(&fs::read_to_string(hooks_path).unwrap()).unwrap();

        assert!(json["hooks"]["UserPromptSubmit"].is_array());
        assert!(json["hooks"]["PermissionRequest"].is_array());
        assert!(json["hooks"]["Stop"].is_array());
        assert_eq!(
            json["hooks"]["Stop"][0]["command"],
            format!("node {}", script_path.display())
        );
    }
}
```

- [ ] **Step 5: Run hook installer tests to verify failure**

Run:

```bash
cargo test --manifest-path app/src-tauri/Cargo.toml hook_installer::tests
```

Expected: test fails because installer is not implemented.

- [ ] **Step 6: Implement hook installer**

Replace `install_hooks`:

```rust
pub fn install_hooks(hooks_path: &Path, hook_script_path: &Path) -> Result<(), String> {
    if let Some(parent) = hooks_path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }

    let mut root: Value = if hooks_path.exists() {
        serde_json::from_str(&fs::read_to_string(hooks_path).map_err(|err| err.to_string())?)
            .unwrap_or_else(|_| json!({}))
    } else {
        json!({})
    };

    if root.get("hooks").is_none() {
        root["hooks"] = json!({});
    }

    let command = format!("node {}", hook_script_path.display());
    for event_name in ["UserPromptSubmit", "PermissionRequest", "Stop"] {
        root["hooks"][event_name] = json!([{ "command": command }]);
    }

    fs::write(
        hooks_path,
        serde_json::to_string_pretty(&root).map_err(|err| err.to_string())?,
    )
    .map_err(|err| err.to_string())
}
```

- [ ] **Step 7: Add hook forwarding script**

Create `app/scripts/codex-status-hook.js`:

```javascript
#!/usr/bin/env node

const http = require("node:http");

let body = "";
process.stdin.setEncoding("utf8");
process.stdin.on("data", (chunk) => {
  body += chunk;
});

process.stdin.on("end", () => {
  const request = http.request(
    {
      hostname: "127.0.0.1",
      port: 17321,
      path: "/codex-hook",
      method: "POST",
      headers: {
        "content-type": "application/json",
        "content-length": Buffer.byteLength(body),
      },
      timeout: 400,
    },
    (response) => {
      response.resume();
      response.on("end", () => process.exit(0));
    }
  );

  request.on("error", () => process.exit(0));
  request.on("timeout", () => {
    request.destroy();
    process.exit(0);
  });
  request.write(body);
  request.end();
});
```

- [ ] **Step 8: Register modules and run tests**

Add modules to `lib.rs`:

```rust
mod hook_installer;
mod settings;
```

Run:

```bash
cargo test --manifest-path app/src-tauri/Cargo.toml settings::tests hook_installer::tests
```

Expected: settings and hook installer tests pass.

- [ ] **Step 9: Commit**

Run:

```bash
git add app/src-tauri/src/settings.rs app/src-tauri/src/hook_installer.rs app/scripts/codex-status-hook.js app/src-tauri/src/lib.rs
git commit -m "feat: add settings and codex hook installer"
```

## Task 7: Add Hook HTTP Server And Tauri Commands

**Files:**

- Create: `app/src-tauri/src/hook_server.rs`
- Create: `app/src-tauri/src/dto.rs`
- Modify: `app/src-tauri/src/lib.rs`

- [ ] **Step 1: Add DTOs**

Create `app/src-tauri/src/dto.rs`:

```rust
use crate::domain::{TaskRecord, TaskStatus};
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskDto {
    pub id: String,
    pub title: String,
    pub provider: String,
    pub status: TaskStatus,
    pub viewed: bool,
}

impl From<TaskRecord> for TaskDto {
    fn from(task: TaskRecord) -> Self {
        Self {
            id: task.id,
            title: task.title,
            provider: task.provider,
            status: task.status,
            viewed: task.viewed,
        }
    }
}
```

- [ ] **Step 2: Write hook server handler tests**

Create `app/src-tauri/src/hook_server.rs`:

```rust
use crate::codex_provider::{normalize_payload, CodexHookPayload};
use crate::notifier::{BarkNotifier, OutboundNotification};
use crate::settings::AppSettings;
use crate::task_store::TaskStore;
use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct SharedBackendState {
    pub store: Arc<Mutex<TaskStore>>,
    pub settings: Arc<Mutex<AppSettings>>,
}

pub fn router(state: SharedBackendState) -> Router {
    Router::new().route("/codex-hook", post(handle_codex_hook)).with_state(state)
}

pub async fn handle_codex_hook(
    State(state): State<SharedBackendState>,
    Json(payload): Json<CodexHookPayload>,
) -> StatusCode {
    let _ = (state, payload);
    StatusCode::NOT_IMPLEMENTED
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    #[tokio::test]
    async fn accepts_hook_payload() {
        let state = SharedBackendState {
            store: Arc::new(Mutex::new(TaskStore::new())),
            settings: Arc::new(Mutex::new(AppSettings::default())),
        };
        let app = router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/codex-hook")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"event":"UserPromptSubmit","session_id":"s1","prompt":"修复登录页"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
```

- [ ] **Step 3: Run hook server tests to verify failure**

Run:

```bash
cargo test --manifest-path app/src-tauri/Cargo.toml hook_server::tests
```

Expected: test fails because handler returns `501`.

- [ ] **Step 4: Implement hook handler**

Replace `handle_codex_hook`:

```rust
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
        let settings = state.settings.lock().expect("settings lock poisoned").clone();
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
```

- [ ] **Step 5: Add Tauri commands**

In `app/src-tauri/src/lib.rs`, expose commands and start the hook server. Keep any scaffolded imports that are still needed, then add:

```rust
use crate::dto::TaskDto;
use crate::hook_installer::install_hooks;
use crate::hook_server::{router as hook_router, SharedBackendState};
use crate::settings::{load_settings, save_settings, AppSettings};
use crate::task_store::TaskStore;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;

fn settings_path() -> PathBuf {
    dirs::home_dir()
        .expect("home directory not found")
        .join(".codex-status-light/settings.json")
}

fn codex_hooks_path() -> PathBuf {
    dirs::home_dir()
        .expect("home directory not found")
        .join(".codex/hooks.json")
}

fn hook_script_path() -> PathBuf {
    std::env::current_dir()
        .expect("current directory not found")
        .join("scripts/codex-status-hook.js")
}

#[tauri::command]
fn list_tasks(state: tauri::State<SharedBackendState>) -> Vec<TaskDto> {
    state
        .store
        .lock()
        .expect("task store lock poisoned")
        .visible_tasks()
        .into_iter()
        .map(TaskDto::from)
        .collect()
}

#[tauri::command]
fn mark_task_viewed(task_id: String, state: tauri::State<SharedBackendState>) {
    state
        .store
        .lock()
        .expect("task store lock poisoned")
        .mark_viewed(&task_id, Instant::now());
}

#[tauri::command]
fn get_settings(state: tauri::State<SharedBackendState>) -> AppSettings {
    state.settings.lock().expect("settings lock poisoned").clone()
}

#[tauri::command]
fn save_app_settings(
    settings: AppSettings,
    state: tauri::State<SharedBackendState>,
) -> Result<AppSettings, String> {
    save_settings(&settings_path(), &settings)?;
    *state.settings.lock().expect("settings lock poisoned") = settings.clone();
    Ok(settings)
}

#[tauri::command]
fn install_codex_hooks() -> Result<(), String> {
    install_hooks(&codex_hooks_path(), &hook_script_path())
}

#[tauri::command]
fn get_hook_status() -> String {
    if codex_hooks_path().exists() {
        "installed".to_string()
    } else {
        "not_installed".to_string()
    }
}
```

Keep the scaffolded `run()` function, but add modules:

```rust
mod dto;
mod hook_server;
```

and register commands with:

```rust
tauri::generate_handler![
    list_tasks,
    mark_task_viewed,
    get_settings,
    save_app_settings,
    install_codex_hooks,
    get_hook_status
]
```

Inside `run()`, create shared backend state and start the hook server during setup:

```rust
pub fn run() {
    let state = SharedBackendState {
        store: Arc::new(Mutex::new(TaskStore::new())),
        settings: Arc::new(Mutex::new(load_settings(&settings_path()))),
    };
    let hook_state = state.clone();

    tauri::Builder::default()
        .manage(state)
        .setup(move |_app| {
            tauri::async_runtime::spawn(async move {
                match tokio::net::TcpListener::bind("127.0.0.1:17321").await {
                    Ok(listener) => {
                        let _ = axum::serve(listener, hook_router(hook_state)).await;
                    }
                    Err(err) => {
                        eprintln!("failed to bind hook server: {err}");
                    }
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_tasks,
            mark_task_viewed,
            get_settings,
            save_app_settings,
            install_codex_hooks,
            get_hook_status
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 6: Run Rust tests**

Run:

```bash
cargo test --manifest-path app/src-tauri/Cargo.toml
```

Expected: all Rust tests pass.

- [ ] **Step 7: Commit**

Run:

```bash
git add app/src-tauri/src/hook_server.rs app/src-tauri/src/dto.rs app/src-tauri/src/lib.rs
git commit -m "feat: add codex hook endpoint and tauri commands"
```

## Task 8: Build Frontend Overlay Components

**Files:**

- Create: `app/src/types.ts`
- Create: `app/src/components/TrafficDots.tsx`
- Create: `app/src/components/TaskOverlay.tsx`
- Create: `app/src/components/TaskOverlay.test.tsx`
- Create: `app/src/test/setup.ts`
- Modify: `app/src/styles.css`
- Modify: `app/src/main.tsx`
- Modify: `app/package.json`

- [ ] **Step 1: Configure Vitest setup**

Add to `app/package.json`:

```json
{
  "vitest": {
    "environment": "jsdom",
    "setupFiles": ["src/test/setup.ts"]
  }
}
```

Create `app/src/test/setup.ts`:

```typescript
import "@testing-library/jest-dom/vitest";
```

- [ ] **Step 2: Add frontend types**

Create `app/src/types.ts`:

```typescript
export type TaskStatus =
  | "executing"
  | "needs_permission"
  | "needs_confirmation"
  | "completed"
  | "interrupted";

export type TaskDto = {
  id: string;
  title: string;
  provider: string;
  status: TaskStatus;
  viewed: boolean;
};
```

- [ ] **Step 3: Write failing overlay tests**

Create `app/src/components/TaskOverlay.test.tsx`:

```tsx
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { TaskOverlay } from "./TaskOverlay";
import type { TaskDto } from "../types";

const tasks: TaskDto[] = [
  {
    id: "s1",
    title: "重构登录页权限判断",
    provider: "Codex",
    status: "executing",
    viewed: false,
  },
  {
    id: "s2",
    title: "接入支付回调测试",
    provider: "Codex",
    status: "needs_permission",
    viewed: false,
  },
];

describe("TaskOverlay", () => {
  it("renders task titles and provider marker", () => {
    render(<TaskOverlay tasks={tasks} onTaskViewed={() => {}} />);

    expect(screen.getByText("重构登录页权限判断")).toBeInTheDocument();
    expect(screen.getByText("接入支付回调测试")).toBeInTheDocument();
    expect(screen.getAllByText("Codex")).toHaveLength(2);
  });

  it("calls onTaskViewed when a row is clicked", async () => {
    const onTaskViewed = vi.fn();
    render(<TaskOverlay tasks={tasks} onTaskViewed={onTaskViewed} />);

    await userEvent.click(screen.getByText("接入支付回调测试"));

    expect(onTaskViewed).toHaveBeenCalledWith("s2");
  });

  it("renders nothing when there are no tasks", () => {
    const { container } = render(<TaskOverlay tasks={[]} onTaskViewed={() => {}} />);
    expect(container.firstChild).toBeNull();
  });
});
```

- [ ] **Step 4: Run frontend tests to verify failure**

Run:

```bash
cd app
npm test -- TaskOverlay
```

Expected: tests fail because components do not exist.

- [ ] **Step 5: Implement traffic dots**

Create `app/src/components/TrafficDots.tsx`:

```tsx
import type { TaskStatus } from "../types";

type Props = {
  status: TaskStatus;
};

export function TrafficDots({ status }: Props) {
  return (
    <div className={`traffic-dots traffic-dots--${status}`} aria-label={status}>
      <span className="traffic-dot traffic-dot--green" />
      <span className="traffic-dot traffic-dot--yellow" />
      <span className="traffic-dot traffic-dot--red" />
    </div>
  );
}
```

- [ ] **Step 6: Implement task overlay**

Create `app/src/components/TaskOverlay.tsx`:

```tsx
import type { TaskDto } from "../types";
import { TrafficDots } from "./TrafficDots";

type Props = {
  tasks: TaskDto[];
  onTaskViewed: (taskId: string) => void;
};

function statusLabel(status: TaskDto["status"]) {
  switch (status) {
    case "executing":
      return "正在执行";
    case "needs_permission":
      return "需要权限";
    case "needs_confirmation":
      return "需要确认";
    case "completed":
      return "已完成";
    case "interrupted":
      return "异常中断";
  }
}

export function TaskOverlay({ tasks, onTaskViewed }: Props) {
  if (tasks.length === 0) {
    return null;
  }

  return (
    <aside className="task-overlay" aria-label="Codex task status">
      {tasks.map((task) => (
        <button
          className="task-row"
          key={task.id}
          type="button"
          onClick={() => onTaskViewed(task.id)}
        >
          <TrafficDots status={task.status} />
          <span className="task-copy">
            <span className="task-title">{task.title}</span>
            <span className="task-status">{statusLabel(task.status)}</span>
          </span>
          <span className="provider-pill">{task.provider}</span>
        </button>
      ))}
    </aside>
  );
}
```

- [ ] **Step 7: Style overlay**

Replace or extend `app/src/styles.css`:

```css
:root {
  color: #f8fafc;
  background: transparent;
  font-family: Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
}

body {
  margin: 0;
  min-width: 320px;
  min-height: 120px;
  background: transparent;
}

button {
  font: inherit;
}

.task-overlay {
  position: fixed;
  top: 16px;
  right: 16px;
  width: min(360px, calc(100vw - 32px));
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 10px;
  border-radius: 16px;
  background: rgba(17, 24, 39, 0.94);
  box-shadow: 0 22px 50px rgba(15, 23, 42, 0.28);
}

.task-row {
  width: 100%;
  display: flex;
  align-items: center;
  gap: 10px;
  min-height: 50px;
  padding: 10px;
  border: 0;
  border-radius: 12px;
  color: #f8fafc;
  background: rgba(255, 255, 255, 0.06);
  cursor: pointer;
  text-align: left;
}

.task-row:hover {
  background: rgba(255, 255, 255, 0.1);
}

.traffic-dots {
  display: flex;
  flex: 0 0 auto;
  gap: 5px;
}

.traffic-dot {
  width: 9px;
  height: 9px;
  border-radius: 50%;
  background: #3a3f46;
}

.traffic-dots--executing .traffic-dot--green {
  background: #22c55e;
  box-shadow: 0 0 12px #22c55e;
}

.traffic-dots--needs_permission .traffic-dot--yellow,
.traffic-dots--needs_confirmation .traffic-dot--yellow {
  background: #facc15;
  box-shadow: 0 0 12px #facc15;
}

.traffic-dots--completed .traffic-dot--red {
  background: #ef4444;
  box-shadow: 0 0 12px #ef4444;
}

.traffic-dots--interrupted .traffic-dot--yellow,
.traffic-dots--interrupted .traffic-dot--red {
  animation: pulse-alert 0.85s infinite alternate;
}

.traffic-dots--interrupted .traffic-dot--yellow {
  background: #facc15;
  box-shadow: 0 0 12px #facc15;
}

.traffic-dots--interrupted .traffic-dot--red {
  background: #ef4444;
  box-shadow: 0 0 12px #ef4444;
}

.task-copy {
  min-width: 0;
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.task-title {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 13px;
  font-weight: 650;
}

.task-status {
  color: #aab2bd;
  font-size: 11px;
}

.provider-pill {
  flex: 0 0 auto;
  padding: 3px 7px;
  border: 1px solid rgba(203, 213, 225, 0.26);
  border-radius: 999px;
  color: #cbd5e1;
  font-size: 10px;
}

@keyframes pulse-alert {
  from {
    opacity: 0.38;
  }
  to {
    opacity: 1;
  }
}
```

- [ ] **Step 8: Run frontend tests**

Run:

```bash
cd app
npm test -- TaskOverlay
```

Expected: all overlay tests pass.

- [ ] **Step 9: Commit**

Run:

```bash
git add app/src/types.ts app/src/components app/src/test app/src/styles.css app/package.json
git commit -m "feat: add desktop task overlay"
```

## Task 9: Wire Frontend To Tauri Commands And Settings UI

**Files:**

- Create: `app/src/api.ts`
- Create: `app/src/components/SettingsPanel.tsx`
- Create: `app/src/components/SettingsPanel.test.tsx`
- Modify: `app/src/App.tsx`

- [ ] **Step 1: Add API wrapper**

Create `app/src/api.ts`:

```typescript
import { invoke } from "@tauri-apps/api/core";
import type { TaskDto } from "./types";

export type AppSettings = {
  barkEndpointUrl: string;
  notificationsEnabled: boolean;
  overlayPosition: "top_right" | "top_left";
  startAtLogin: boolean;
};

export async function listTasks(): Promise<TaskDto[]> {
  return invoke<TaskDto[]>("list_tasks");
}

export async function markTaskViewed(taskId: string): Promise<void> {
  await invoke("mark_task_viewed", { taskId });
}

export async function getSettings(): Promise<AppSettings> {
  return invoke<AppSettings>("get_settings");
}

export async function saveSettings(settings: AppSettings): Promise<AppSettings> {
  return invoke<AppSettings>("save_app_settings", { settings });
}

export async function installCodexHooks(): Promise<void> {
  await invoke("install_codex_hooks");
}

export async function getHookStatus(): Promise<"installed" | "not_installed" | "unknown"> {
  return invoke<"installed" | "not_installed" | "unknown">("get_hook_status");
}
```

- [ ] **Step 2: Write settings panel tests**

Create `app/src/components/SettingsPanel.test.tsx`:

```tsx
import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { SettingsPanel } from "./SettingsPanel";

describe("SettingsPanel", () => {
  it("shows Bark URL field and hook status", () => {
    render(
      <SettingsPanel
        settings={{
          barkEndpointUrl: "https://api.day.app/key",
          notificationsEnabled: true,
          overlayPosition: "top_right",
          startAtLogin: false,
        }}
        hookStatus="not_installed"
        onSave={() => {}}
        onInstallHooks={() => {}}
      />
    );

    expect(screen.getByLabelText("Bark URL")).toHaveValue("https://api.day.app/key");
    expect(screen.getByText("Hook: not_installed")).toBeInTheDocument();
  });
});
```

- [ ] **Step 3: Run settings UI tests to verify failure**

Run:

```bash
cd app
npm test -- SettingsPanel
```

Expected: test fails because component does not exist.

- [ ] **Step 4: Implement settings panel**

Create `app/src/components/SettingsPanel.tsx`:

```tsx
import type { AppSettings } from "../api";

type Props = {
  settings: AppSettings;
  hookStatus: "installed" | "not_installed" | "unknown";
  onSave: (settings: AppSettings) => void;
  onInstallHooks: () => void;
};

export function SettingsPanel({ settings, hookStatus, onSave, onInstallHooks }: Props) {
  return (
    <section className="settings-panel" aria-label="Settings">
      <label>
        Bark URL
        <input
          defaultValue={settings.barkEndpointUrl}
          aria-label="Bark URL"
          onBlur={(event) =>
            onSave({
              ...settings,
              barkEndpointUrl: event.currentTarget.value,
              notificationsEnabled: event.currentTarget.value.trim().length > 0,
            })
          }
        />
      </label>
      <label>
        <input type="checkbox" readOnly checked={settings.notificationsEnabled} />
        Notifications
      </label>
      <div>Hook: {hookStatus}</div>
      <button type="button" onClick={onInstallHooks}>
        Install Codex hooks
      </button>
    </section>
  );
}
```

- [ ] **Step 5: Wire app shell**

Replace `app/src/App.tsx`:

```tsx
import { useEffect, useState } from "react";
import {
  getHookStatus,
  getSettings,
  installCodexHooks,
  listTasks,
  markTaskViewed,
  saveSettings,
  type AppSettings,
} from "./api";
import { SettingsPanel } from "./components/SettingsPanel";
import { TaskOverlay } from "./components/TaskOverlay";
import type { TaskDto } from "./types";
import "./styles.css";

export default function App() {
  const [tasks, setTasks] = useState<TaskDto[]>([]);
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [hookStatus, setHookStatus] = useState<"installed" | "not_installed" | "unknown">("unknown");
  const [showSettings, setShowSettings] = useState(false);

  useEffect(() => {
    let cancelled = false;

    async function refresh() {
      const [nextTasks, nextSettings, nextHookStatus] = await Promise.all([
        listTasks(),
        getSettings(),
        getHookStatus(),
      ]);
      if (!cancelled) {
        setTasks(nextTasks);
        setSettings(nextSettings);
        setHookStatus(nextHookStatus);
      }
    }

    refresh();
    const interval = window.setInterval(refresh, 1000);
    return () => {
      cancelled = true;
      window.clearInterval(interval);
    };
  }, []);

  async function handleTaskViewed(taskId: string) {
    await markTaskViewed(taskId);
    setTasks(await listTasks());
  }

  async function handleSaveSettings(nextSettings: AppSettings) {
    setSettings(await saveSettings(nextSettings));
  }

  async function handleInstallHooks() {
    await installCodexHooks();
    setHookStatus(await getHookStatus());
  }

  return (
    <>
      <TaskOverlay tasks={tasks} onTaskViewed={handleTaskViewed} />
      {showSettings && settings ? (
        <SettingsPanel
          settings={settings}
          hookStatus={hookStatus}
          onSave={handleSaveSettings}
          onInstallHooks={handleInstallHooks}
        />
      ) : null}
      <button className="settings-toggle" type="button" onClick={() => setShowSettings((value) => !value)}>
        Settings
      </button>
    </>
  );
}
```

- [ ] **Step 6: Run frontend tests**

Run:

```bash
cd app
npm test
npm run build
```

Expected: tests and build pass.

- [ ] **Step 7: Commit**

Run:

```bash
git add app/src/api.ts app/src/App.tsx app/src/components/SettingsPanel.tsx app/src/components/SettingsPanel.test.tsx
git commit -m "feat: wire overlay to tauri commands"
```

## Task 10: End-To-End Local Verification

**Files:**

- Modify: `app/README.md`

- [ ] **Step 1: Add README usage**

Create or replace `app/README.md`:

```markdown
# Codex Status Light

Lightweight macOS desktop status light for Codex tasks.

## Development

```bash
npm install
npm run tauri dev
```

## Bark

Paste your Bark endpoint URL into settings. Notifications are one line:

```text
状态：任务名
```

## Codex Hooks

Use the app settings action to install user-level hooks into `~/.codex/hooks.json`. The hook script forwards Codex hook JSON to `http://127.0.0.1:17321/codex-hook`.
```

- [ ] **Step 2: Run full verification**

Run:

```bash
cd app
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
```

Expected: all tests pass and frontend builds.

- [ ] **Step 3: Start dev app**

Run:

```bash
cd app
npm run tauri dev
```

Expected: Tauri opens the app without build errors.

- [ ] **Step 4: Simulate a Codex prompt event**

In a second terminal, run:

```bash
curl -sS -X POST http://127.0.0.1:17321/codex-hook \
  -H 'content-type: application/json' \
  -d '{"event":"UserPromptSubmit","session_id":"manual-1","prompt":"帮我修复移动端导航遮挡并跑测试"}'
```

Expected: HTTP 200 and the overlay shows `修复移动端导航遮挡` with green lit.

- [ ] **Step 5: Simulate permission event**

Run:

```bash
curl -sS -X POST http://127.0.0.1:17321/codex-hook \
  -H 'content-type: application/json' \
  -d '{"event":"PermissionRequest","session_id":"manual-1"}'
```

Expected: HTTP 200 and the overlay shows the task with yellow lit. If Bark is configured, iPhone receives `需要权限：修复移动端导航遮挡`.

- [ ] **Step 6: Simulate completed event**

Run:

```bash
curl -sS -X POST http://127.0.0.1:17321/codex-hook \
  -H 'content-type: application/json' \
  -d '{"event":"Stop","session_id":"manual-1","message":"已完成修改并通过测试。"}'
```

Expected: HTTP 200 and the overlay shows the task with red lit. If Bark is configured, iPhone receives `已完成：修复移动端导航遮挡`.

- [ ] **Step 7: Commit**

Run:

```bash
git add app/README.md
git commit -m "docs: add local verification guide"
```

## Self-Review Checklist

- Spec coverage: tasks cover scaffold, Codex provider, five states, title derivation, desktop overlay, Bark notification, settings, hook installation, hook HTTP transport, disappearance behavior, and tests.
- Placeholder scan: the plan contains no `TBD`, no `TODO`, and no "implement later" steps.
- Type consistency: backend `TaskStatus` serializes to snake_case; frontend `TaskStatus` uses the same snake_case strings; DTO field names serialize to camelCase for TypeScript.
- Known risk: actual Codex hook payload field names may differ from the simplified `CodexHookPayload`. During implementation, inspect real hook payloads and extend `CodexHookPayload` while keeping these tests as the product contract.
