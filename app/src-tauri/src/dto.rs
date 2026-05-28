use serde::Serialize;

use crate::domain::{TaskRecord, TaskStatus};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::TaskStatus;

    #[test]
    fn task_dto_serializes_as_camel_case() {
        let dto = TaskDto {
            id: "s1".into(),
            title: "修复登录页".into(),
            provider: "Codex".into(),
            status: TaskStatus::Executing,
            viewed: false,
        };

        let value = serde_json::to_value(dto).unwrap();

        assert_eq!(value["id"], "s1");
        assert_eq!(value["title"], "修复登录页");
        assert_eq!(value["provider"], "Codex");
        assert_eq!(value["status"], "executing");
        assert_eq!(value["viewed"], false);
    }
}
