import { invoke } from "@tauri-apps/api/core";
import type { AppSettings, HookStatus, TaskDto } from "./types";

type RawAppSettings = {
  bark_endpoint_url?: string;
  barkEndpointUrl?: string;
  codex_hooks_installed?: boolean;
  codexHooksInstalled?: boolean;
  notifications_enabled?: boolean;
  notificationsEnabled?: boolean;
  overlay_position?: AppSettings["overlayPosition"];
  overlayPosition?: AppSettings["overlayPosition"];
  start_at_login?: boolean;
  startAtLogin?: boolean;
};

function normalizeSettings(settings: RawAppSettings): AppSettings {
  return {
    barkEndpointUrl: settings.barkEndpointUrl ?? settings.bark_endpoint_url ?? "",
    codexHooksInstalled:
      settings.codexHooksInstalled ?? settings.codex_hooks_installed ?? false,
    notificationsEnabled:
      settings.notificationsEnabled ?? settings.notifications_enabled ?? false,
    overlayPosition: settings.overlayPosition ?? settings.overlay_position ?? "top_right",
    startAtLogin: settings.startAtLogin ?? settings.start_at_login ?? false,
  };
}

function serializeSettings(settings: AppSettings): RawAppSettings {
  return {
    bark_endpoint_url: settings.barkEndpointUrl,
    codex_hooks_installed: settings.codexHooksInstalled,
    notifications_enabled: settings.notificationsEnabled,
    overlay_position: settings.overlayPosition,
    start_at_login: settings.startAtLogin,
  };
}

export async function listTasks(): Promise<TaskDto[]> {
  return invoke<TaskDto[]>("list_tasks");
}

export async function markTaskViewed(taskId: string): Promise<void> {
  await invoke("mark_task_viewed", { taskId });
}

export async function getSettings(): Promise<AppSettings> {
  return normalizeSettings(await invoke<RawAppSettings>("get_settings"));
}

export async function saveAppSettings(settings: AppSettings): Promise<AppSettings> {
  return normalizeSettings(
    await invoke<RawAppSettings>("save_app_settings", {
      settings: serializeSettings(settings),
    }),
  );
}

export async function installCodexHooks(): Promise<AppSettings> {
  return normalizeSettings(await invoke<RawAppSettings>("install_codex_hooks"));
}

export async function getHookStatus(): Promise<HookStatus> {
  return invoke<HookStatus>("get_hook_status");
}

export async function setOverlayWindowBounds(
  settingsOpen: boolean,
  taskCount: number,
): Promise<void> {
  await invoke("set_overlay_window_bounds", { settingsOpen, taskCount });
}
