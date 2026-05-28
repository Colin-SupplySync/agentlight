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

export type OverlayPosition = "top_right" | "top_left";

export type HookStatus = "installed" | "not_installed" | string;

export type AppSettings = {
  barkEndpointUrl: string;
  codexHooksInstalled: boolean;
  notificationsEnabled: boolean;
  overlayPosition: OverlayPosition;
  startAtLogin: boolean;
};
