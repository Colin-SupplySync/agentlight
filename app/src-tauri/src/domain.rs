use serde::{Deserialize, Serialize};

use crate::title::derive_title;

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
    match event {
        NormalizedEvent::UserPromptSubmit { session_id, prompt } => TaskRecord {
            id: session_id,
            title: derive_title(&prompt),
            provider: "Codex".to_string(),
            status: TaskStatus::Executing,
            viewed: false,
        },
        NormalizedEvent::PermissionRequested { session_id } => {
            transition(existing, session_id, TaskStatus::NeedsPermission)
        }
        NormalizedEvent::ConfirmationRequested { session_id } => {
            transition(existing, session_id, TaskStatus::NeedsConfirmation)
        }
        NormalizedEvent::Completed { session_id } => {
            transition(existing, session_id, TaskStatus::Completed)
        }
        NormalizedEvent::Interrupted {
            session_id,
            reason: _,
        } => transition(existing, session_id, TaskStatus::Interrupted),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_creates_executing_task() {
        let task = reduce_task(
            None,
            NormalizedEvent::UserPromptSubmit {
                session_id: "s1".into(),
                prompt: "帮我修复移动端导航遮挡并跑测试".into(),
            },
        );

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

        let task = reduce_task(
            Some(existing),
            NormalizedEvent::PermissionRequested {
                session_id: "s1".into(),
            },
        );

        assert_eq!(task.title, "修复移动端导航遮挡");
        assert_eq!(task.status, TaskStatus::NeedsPermission);
    }

    #[test]
    fn completed_sets_red_state() {
        let task = reduce_task(
            None,
            NormalizedEvent::Completed {
                session_id: "s2".into(),
            },
        );

        assert_eq!(task.title, "Codex task");
        assert_eq!(task.status, TaskStatus::Completed);
    }

    #[test]
    fn confirmation_sets_yellow_state() {
        let task = reduce_task(
            None,
            NormalizedEvent::ConfirmationRequested {
                session_id: "s2".into(),
            },
        );

        assert_eq!(task.status, TaskStatus::NeedsConfirmation);
    }

    #[test]
    fn interrupted_sets_flashing_state() {
        let task = reduce_task(
            None,
            NormalizedEvent::Interrupted {
                session_id: "s3".into(),
                reason: "connection lost".into(),
            },
        );

        assert_eq!(task.status, TaskStatus::Interrupted);
    }
}
