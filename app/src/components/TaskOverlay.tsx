import type { TaskDto } from "../types";
import "../styles.css";
import { StatusChip } from "./StatusChip";

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

function taskLabel(task: TaskDto) {
  return `${statusLabel(task.status)}：${task.title}`;
}

export function TaskOverlay({ tasks, onTaskViewed }: Props) {
  if (tasks.length === 0) {
    return null;
  }

  return (
    <aside className="task-overlay" aria-label="Codex task status">
      {tasks.map((task) => (
        <div
          className={`task-row task-row--${task.status}`}
          key={task.id}
          role="button"
          tabIndex={0}
          onClick={() => onTaskViewed(task.id)}
          onKeyDown={(event) => {
            if (event.key === "Enter" || event.key === " ") {
              event.preventDefault();
              onTaskViewed(task.id);
            }
          }}
        >
          <StatusChip status={task.status} />
          <span className="task-copy">
            <span className="task-title">{taskLabel(task)}</span>
          </span>
          <span className="provider-marker">{task.provider}</span>
        </div>
      ))}
    </aside>
  );
}
