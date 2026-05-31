import type { TaskStatus } from "../types";

type Props = {
  status: TaskStatus;
};

const chipLabels: Record<TaskStatus, string> = {
  executing: "RUN",
  needs_permission: "AUTH",
  needs_confirmation: "ASK",
  completed: "DONE",
  interrupted: "ERR",
};

const statusLabels: Record<TaskStatus, string> = {
  executing: "正在执行",
  needs_permission: "需要权限",
  needs_confirmation: "需要确认",
  completed: "已完成",
  interrupted: "异常中断",
};

export function StatusChip({ status }: Props) {
  return (
    <span
      className={`status-chip status-chip--${status}`}
      aria-label={`状态芯片：${statusLabels[status]}`}
    >
      {chipLabels[status]}
    </span>
  );
}
