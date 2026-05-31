use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use crate::domain::{reduce_task, NormalizedEvent, TaskRecord, TaskStatus};

const EXECUTING_IDLE_EXPIRATION: Duration = Duration::from_secs(12 * 60 * 60);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotificationRequest {
    pub task_id: String,
    pub status: TaskStatus,
    pub title: String,
}

#[derive(Default)]
pub struct TaskStore {
    tasks: HashMap<String, StoredTask>,
    notified: HashSet<(String, TaskStatus)>,
    next_sequence: u64,
}

struct StoredTask {
    record: TaskRecord,
    viewed_at: Option<Instant>,
    updated_at: Instant,
    sequence: u64,
}

impl TaskStore {
    pub fn new() -> Self {
        Self::default()
    }

    #[cfg(test)]
    pub fn apply_event(&mut self, event: NormalizedEvent) -> Option<NotificationRequest> {
        self.apply_event_with_title(event, None)
    }

    pub fn apply_event_with_title(
        &mut self,
        event: NormalizedEvent,
        title_hint: Option<String>,
    ) -> Option<NotificationRequest> {
        self.apply_event_at(event, Instant::now(), title_hint)
    }

    fn apply_event_at(
        &mut self,
        event: NormalizedEvent,
        now: Instant,
        title_hint: Option<String>,
    ) -> Option<NotificationRequest> {
        let task_id = session_id(&event).to_string();
        if matches!(event, NormalizedEvent::UserPromptSubmit { .. }) {
            self.notified
                .retain(|(notified_task_id, _)| notified_task_id != &task_id);
        }

        let existing_task = self.tasks.remove(&task_id);
        let sequence = existing_task.as_ref().map_or_else(
            || {
                let sequence = self.next_sequence;
                self.next_sequence += 1;
                sequence
            },
            |task| task.sequence,
        );
        let existing = existing_task.map(|task| task.record);
        let mut task = reduce_task(existing, event);
        if let Some(title) = clean_title_hint(title_hint) {
            task.title = title;
        }
        let notification = self.notification_for(&task);

        self.tasks.insert(
            task_id,
            StoredTask {
                record: task,
                viewed_at: None,
                updated_at: now,
                sequence,
            },
        );

        notification
    }

    pub fn mark_viewed(&mut self, task_id: &str, now: Instant) {
        if let Some(task) = self.tasks.get_mut(task_id) {
            task.record.viewed = true;
            task.viewed_at = Some(now);
        }
    }

    pub fn remove_expired_viewed(&mut self, now: Instant, delay: Duration) {
        let expired: Vec<String> = self
            .tasks
            .iter()
            .filter_map(|(task_id, task)| {
                let viewed_expired = task.record.viewed
                    && task
                        .viewed_at
                        .is_some_and(|viewed_at| now.duration_since(viewed_at) >= delay);
                let terminal_expired = matches!(
                    task.record.status,
                    TaskStatus::Completed | TaskStatus::Interrupted
                ) && now.duration_since(task.updated_at) >= delay;
                let executing_idle_expired = task.record.status == TaskStatus::Executing
                    && now.duration_since(task.updated_at) >= EXECUTING_IDLE_EXPIRATION;

                if viewed_expired || terminal_expired || executing_idle_expired {
                    Some(task_id.clone())
                } else {
                    None
                }
            })
            .collect();

        for task_id in expired {
            self.tasks.remove(&task_id);
            self.notified
                .retain(|(notified_task_id, _)| notified_task_id != &task_id);
        }
    }

    pub fn has_task(&self, task_id: &str) -> bool {
        self.tasks.contains_key(task_id)
    }

    pub fn visible_tasks(&self) -> Vec<TaskRecord> {
        let mut tasks: Vec<&StoredTask> = self.tasks.values().collect();
        tasks.sort_by_key(|task| task.sequence);
        tasks.into_iter().map(|task| task.record.clone()).collect()
    }
}

impl TaskStore {
    fn notification_for(&mut self, task: &TaskRecord) -> Option<NotificationRequest> {
        if !matches!(
            task.status,
            TaskStatus::NeedsPermission
                | TaskStatus::NeedsConfirmation
                | TaskStatus::Completed
                | TaskStatus::Interrupted
        ) {
            return None;
        }

        let key = (task.id.clone(), task.status.clone());
        if !self.notified.insert(key) {
            return None;
        }

        Some(NotificationRequest {
            task_id: task.id.clone(),
            status: task.status.clone(),
            title: task.title.clone(),
        })
    }
}

fn clean_title_hint(title_hint: Option<String>) -> Option<String> {
    title_hint
        .map(|title| title.trim().to_string())
        .filter(|title| !title.is_empty())
}

fn session_id(event: &NormalizedEvent) -> &str {
    match event {
        NormalizedEvent::UserPromptSubmit { session_id, .. }
        | NormalizedEvent::PermissionRequested { session_id }
        | NormalizedEvent::ConfirmationRequested { session_id }
        | NormalizedEvent::Completed { session_id }
        | NormalizedEvent::Interrupted { session_id, .. } => session_id,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prompt(session_id: &str) -> NormalizedEvent {
        NormalizedEvent::UserPromptSubmit {
            session_id: session_id.into(),
            prompt: "帮我修复移动端导航遮挡并跑测试".into(),
        }
    }

    fn permission(session_id: &str) -> NormalizedEvent {
        NormalizedEvent::PermissionRequested {
            session_id: session_id.into(),
        }
    }

    fn confirmation(session_id: &str) -> NormalizedEvent {
        NormalizedEvent::ConfirmationRequested {
            session_id: session_id.into(),
        }
    }

    fn completed(session_id: &str) -> NormalizedEvent {
        NormalizedEvent::Completed {
            session_id: session_id.into(),
        }
    }

    fn interrupted(session_id: &str) -> NormalizedEvent {
        NormalizedEvent::Interrupted {
            session_id: session_id.into(),
            reason: "connection lost".into(),
        }
    }

    #[test]
    fn executing_does_not_notify() {
        let mut store = TaskStore::new();

        let notification = store.apply_event(prompt("s1"));

        assert_eq!(notification, None);
        assert_eq!(store.visible_tasks()[0].status, TaskStatus::Executing);
    }

    #[test]
    fn permission_notifies_once_per_state() {
        let mut store = TaskStore::new();
        store.apply_event(prompt("s1"));

        let first = store.apply_event(permission("s1"));
        let duplicate = store.apply_event(permission("s1"));
        let confirmation_notice = store.apply_event(confirmation("s1"));
        let repeated_confirmation_notice = store.apply_event(confirmation("s1"));

        assert_eq!(
            first,
            Some(NotificationRequest {
                task_id: "s1".into(),
                status: TaskStatus::NeedsPermission,
                title: "修复移动端导航遮挡".into(),
            })
        );
        assert_eq!(duplicate, None);
        assert_eq!(
            confirmation_notice,
            Some(NotificationRequest {
                task_id: "s1".into(),
                status: TaskStatus::NeedsConfirmation,
                title: "修复移动端导航遮挡".into(),
            })
        );
        assert_eq!(repeated_confirmation_notice, None);
    }

    #[test]
    fn title_hint_overrides_prompt_title_and_updates_on_later_status() {
        let mut store = TaskStore::new();
        store.apply_event_with_title(prompt("s1"), Some("调研红绿黄灯项目".into()));
        assert_eq!(store.visible_tasks()[0].title, "调研红绿黄灯项目");

        let notification = store.apply_event_with_title(completed("s1"), Some("红绿灯 MVP".into()));

        assert_eq!(store.visible_tasks()[0].title, "红绿灯 MVP");
        assert_eq!(
            notification,
            Some(NotificationRequest {
                task_id: "s1".into(),
                status: TaskStatus::Completed,
                title: "红绿灯 MVP".into(),
            })
        );
    }

    #[test]
    fn completed_disappears_after_viewed_delay() {
        let mut store = TaskStore::new();
        store.apply_event(prompt("s1"));
        store.apply_event(completed("s1"));

        let viewed_at = Instant::now();
        store.mark_viewed("s1", viewed_at);
        store.remove_expired_viewed(viewed_at, Duration::from_secs(0));

        assert!(store.visible_tasks().is_empty());
    }

    #[test]
    fn completed_disappears_after_terminal_delay_without_being_viewed() {
        let mut store = TaskStore::new();
        store.apply_event(prompt("s1"));
        store.apply_event(completed("s1"));

        store.remove_expired_viewed(Instant::now(), Duration::from_secs(0));

        assert!(store.visible_tasks().is_empty());
    }

    #[test]
    fn yellow_states_disappear_after_viewed_delay() {
        let mut store = TaskStore::new();
        store.apply_event(prompt("s1"));
        store.apply_event(permission("s1"));

        let viewed_at = Instant::now();
        store.mark_viewed("s1", viewed_at);
        store.remove_expired_viewed(viewed_at + Duration::from_secs(14), Duration::from_secs(15));

        let tasks = store.visible_tasks();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].status, TaskStatus::NeedsPermission);
        assert!(tasks[0].viewed);

        store.remove_expired_viewed(viewed_at + Duration::from_secs(15), Duration::from_secs(15));

        assert!(store.visible_tasks().is_empty());
    }

    #[test]
    fn yellow_states_stay_visible_until_viewed() {
        let mut store = TaskStore::new();
        store.apply_event(prompt("s1"));
        store.apply_event(permission("s1"));

        store.remove_expired_viewed(
            Instant::now() + Duration::from_secs(60 * 60),
            Duration::from_secs(15),
        );

        let tasks = store.visible_tasks();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].status, TaskStatus::NeedsPermission);
        assert!(!tasks[0].viewed);
    }

    #[test]
    fn executing_disappears_after_viewed_delay() {
        let mut store = TaskStore::new();
        store.apply_event(prompt("s1"));

        let viewed_at = Instant::now();
        store.mark_viewed("s1", viewed_at);
        store.remove_expired_viewed(viewed_at + Duration::from_secs(14), Duration::from_secs(15));
        assert_eq!(store.visible_tasks().len(), 1);

        store.remove_expired_viewed(viewed_at + Duration::from_secs(15), Duration::from_secs(15));

        assert!(store.visible_tasks().is_empty());
    }

    #[test]
    fn stale_executing_disappears_after_idle_limit_without_being_viewed() {
        let mut store = TaskStore::new();
        let started_at = Instant::now();
        store.apply_event_at(prompt("s1"), started_at, None);

        store.remove_expired_viewed(
            started_at + EXECUTING_IDLE_EXPIRATION - Duration::from_secs(1),
            Duration::from_secs(15),
        );
        assert_eq!(store.visible_tasks().len(), 1);

        store.remove_expired_viewed(
            started_at + EXECUTING_IDLE_EXPIRATION,
            Duration::from_secs(15),
        );

        assert!(store.visible_tasks().is_empty());
    }

    #[test]
    fn long_running_executing_task_stays_visible_after_ten_hours() {
        let mut store = TaskStore::new();
        let started_at = Instant::now();
        store.apply_event_at(prompt("s1"), started_at, None);

        store.remove_expired_viewed(
            started_at + Duration::from_secs(10 * 60 * 60),
            Duration::from_secs(15),
        );

        let tasks = store.visible_tasks();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].status, TaskStatus::Executing);
    }

    #[test]
    fn interrupted_disappears_after_viewed_delay() {
        let mut store = TaskStore::new();
        store.apply_event(prompt("s1"));
        store.apply_event(interrupted("s1"));

        let viewed_at = Instant::now();
        store.mark_viewed("s1", viewed_at);
        store.remove_expired_viewed(viewed_at, Duration::from_secs(0));

        assert!(store.visible_tasks().is_empty());
    }

    #[test]
    fn disappeared_task_notifies_again_when_started_again() {
        let mut store = TaskStore::new();
        let viewed_at = Instant::now();
        store.apply_event(prompt("s1"));
        assert!(store.apply_event(completed("s1")).is_some());

        store.mark_viewed("s1", viewed_at);
        store.remove_expired_viewed(viewed_at + Duration::from_secs(15), Duration::from_secs(15));
        assert!(store.visible_tasks().is_empty());

        store.apply_event(prompt("s1"));
        assert!(store.apply_event(completed("s1")).is_some());
    }

    #[test]
    fn new_prompt_in_same_session_allows_status_notifications_again() {
        let mut store = TaskStore::new();
        store.apply_event(prompt("s1"));
        assert!(store.apply_event(permission("s1")).is_some());

        store.apply_event(prompt("s1"));

        assert!(store.apply_event(permission("s1")).is_some());
        assert!(store.apply_event(completed("s1")).is_some());
    }

    #[test]
    fn visible_tasks_keep_start_order() {
        let mut store = TaskStore::new();
        store.apply_event(prompt("s2"));
        store.apply_event(prompt("s1"));
        store.apply_event(prompt("s3"));

        let ids: Vec<String> = store
            .visible_tasks()
            .into_iter()
            .map(|task| task.id)
            .collect();

        assert_eq!(ids, vec!["s2", "s1", "s3"]);
    }

    #[test]
    fn completed_task_waits_until_viewed_delay_elapses() {
        let mut store = TaskStore::new();
        let viewed_at = Instant::now();
        store.apply_event(prompt("s1"));
        store.apply_event(completed("s1"));

        store.mark_viewed("s1", viewed_at);
        store.remove_expired_viewed(viewed_at + Duration::from_secs(14), Duration::from_secs(15));
        assert_eq!(store.visible_tasks().len(), 1);

        store.remove_expired_viewed(viewed_at + Duration::from_secs(15), Duration::from_secs(15));
        assert!(store.visible_tasks().is_empty());
    }
}
