import type { TaskDto } from "../types";
import "../styles.css";
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
          <span className="provider-marker">{task.provider}</span>
        </button>
      ))}
    </aside>
  );
}
