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
