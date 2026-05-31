import { useCallback, useEffect, useRef, useState } from "react";
import type { PointerEvent as ReactPointerEvent } from "react";
import {
  getHookStatus,
  getSettings,
  installCodexHooks,
  listTasks,
  markTaskViewed,
  saveAppSettings,
  setOverlayWindowBounds,
} from "./api";
import { SettingsPanel } from "./components/SettingsPanel";
import { TaskOverlay } from "./components/TaskOverlay";
import { canStartSurfaceDrag, movedPastDragThreshold } from "./drag";
import "./styles.css";
import type { AppSettings, HookStatus, TaskDto } from "./types";

const defaultSettings: AppSettings = {
  barkEndpointUrl: "",
  codexHooksInstalled: false,
  notificationsEnabled: false,
  overlayPosition: "top_right",
  startAtLogin: false,
};

async function startWindowDrag() {
  if (!("__TAURI_INTERNALS__" in window)) {
    return;
  }

  try {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    await getCurrentWindow().startDragging();
  } catch (error) {
    console.warn("Failed to drag overlay window", error);
  }
}

type PendingDrag = {
  pointerId: number;
  startX: number;
  startY: number;
};

function App() {
  const mountedRef = useRef(false);
  const pendingDragRef = useRef<PendingDrag | null>(null);
  const [tasks, setTasks] = useState<TaskDto[]>([]);
  const [settings, setSettings] = useState<AppSettings>(defaultSettings);
  const [hookStatus, setHookStatus] = useState<HookStatus>("not_installed");
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [saving, setSaving] = useState(false);
  const [installing, setInstalling] = useState(false);

  useEffect(() => {
    mountedRef.current = true;

    return () => {
      mountedRef.current = false;
    };
  }, []);

  useEffect(() => {
    function handlePointerMove(event: PointerEvent) {
      const pendingDrag = pendingDragRef.current;
      if (!pendingDrag || event.pointerId !== pendingDrag.pointerId) {
        return;
      }

      if (!movedPastDragThreshold(
        pendingDrag.startX,
        pendingDrag.startY,
        event.clientX,
        event.clientY,
      )) {
        return;
      }

      pendingDragRef.current = null;
      startWindowDrag();
    }

    function clearPendingDrag(event: PointerEvent) {
      if (pendingDragRef.current?.pointerId === event.pointerId) {
        pendingDragRef.current = null;
      }
    }

    window.addEventListener("pointermove", handlePointerMove);
    window.addEventListener("pointerup", clearPendingDrag);
    window.addEventListener("pointercancel", clearPendingDrag);

    return () => {
      window.removeEventListener("pointermove", handlePointerMove);
      window.removeEventListener("pointerup", clearPendingDrag);
      window.removeEventListener("pointercancel", clearPendingDrag);
    };
  }, []);

  const refreshTasks = useCallback(async (canUpdate = () => mountedRef.current) => {
    try {
      const nextTasks = await listTasks();
      if (canUpdate()) {
        setTasks(nextTasks);
      }
    } catch (error) {
      console.warn("Failed to refresh tasks", error);
    }
  }, []);

  const refreshSettings = useCallback(async (canUpdate = () => mountedRef.current) => {
    try {
      const [nextSettings, nextHookStatus] = await Promise.all([
        getSettings(),
        getHookStatus(),
      ]);
      if (canUpdate()) {
        setSettings(nextSettings);
        setHookStatus(nextHookStatus);
      }
    } catch (error) {
      console.warn("Failed to refresh settings", error);
    }
  }, []);

  useEffect(() => {
    let cancelled = false;
    const canUpdate = () => !cancelled && mountedRef.current;

    refreshTasks(canUpdate);
    if (!settingsOpen) {
      refreshSettings(canUpdate);
    }

    const intervalId = window.setInterval(() => {
      refreshTasks(canUpdate);
      if (!settingsOpen) {
        refreshSettings(canUpdate);
      }
    }, 2500);

    return () => {
      cancelled = true;
      window.clearInterval(intervalId);
    };
  }, [refreshSettings, refreshTasks, settingsOpen]);

  useEffect(() => {
    setOverlayWindowBounds(settingsOpen, tasks.length).catch((error) => {
      console.warn("Failed to sync overlay window bounds", error);
    });
  }, [settingsOpen, tasks.length]);

  async function handleTaskViewed(taskId: string) {
    await markTaskViewed(taskId);
    await refreshTasks();
  }

  async function handleSaveSettings(nextSettings: AppSettings) {
    setSaving(true);
    try {
      const savedSettings = await saveAppSettings(nextSettings);
      setSettings(savedSettings);
    } finally {
      setSaving(false);
    }
  }

  async function handleInstallHooks() {
    setInstalling(true);
    try {
      const installedSettings = await installCodexHooks();
      setSettings(installedSettings);
      setHookStatus(await getHookStatus());
    } finally {
      setInstalling(false);
    }
  }

  function handleSurfacePointerDown(event: ReactPointerEvent<HTMLElement>) {
    if (event.button !== 0 || !canStartSurfaceDrag(event.target)) {
      return;
    }

    pendingDragRef.current = {
      pointerId: event.pointerId,
      startX: event.clientX,
      startY: event.clientY,
    };
  }

  const hasTasks = tasks.length > 0;
  const shouldShowShell = hasTasks || settingsOpen;

  return (
    <main
      className={[
        "app-shell",
        settingsOpen ? "app-shell--settings-open" : "",
        shouldShowShell ? "" : "app-shell--hidden",
      ].filter(Boolean).join(" ")}
      onPointerDown={handleSurfacePointerDown}
    >
      {shouldShowShell ? (
        <button
          className="settings-button"
          type="button"
          aria-label="Settings"
          title="Settings"
          onClick={() => setSettingsOpen((isOpen) => !isOpen)}
        >
          <svg aria-hidden="true" viewBox="0 0 24 24">
            <path d="M4 7h10" />
            <path d="M18 7h2" />
            <path d="M4 17h2" />
            <path d="M10 17h10" />
            <circle cx="16" cy="7" r="2" />
            <circle cx="8" cy="17" r="2" />
          </svg>
        </button>
      ) : null}

      {settingsOpen ? (
        <SettingsPanel
          hookStatus={hookStatus}
          installing={installing}
          saving={saving}
          settings={settings}
          onInstallHooks={handleInstallHooks}
          onSaveSettings={handleSaveSettings}
        />
      ) : null}

      {hasTasks ? <TaskOverlay tasks={tasks} onTaskViewed={handleTaskViewed} /> : null}
    </main>
  );
}

export default App;
