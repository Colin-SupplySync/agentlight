import { useCallback, useEffect, useRef, useState } from "react";
import {
  getHookStatus,
  getSettings,
  installCodexHooks,
  listTasks,
  markTaskViewed,
  saveAppSettings,
} from "./api";
import { SettingsPanel } from "./components/SettingsPanel";
import { TaskOverlay } from "./components/TaskOverlay";
import "./styles.css";
import type { AppSettings, HookStatus, TaskDto } from "./types";

const defaultSettings: AppSettings = {
  barkEndpointUrl: "",
  codexHooksInstalled: false,
  notificationsEnabled: false,
  overlayPosition: "top_right",
  startAtLogin: false,
};

function desiredWindowSize(settingsOpen: boolean, taskCount: number) {
  const hasTasks = taskCount > 0;
  const width = hasTasks ? 392 : settingsOpen ? 316 : 104;
  const settingsHeight = settingsOpen ? 152 : 0;
  const taskHeight = hasTasks ? 18 + taskCount * 48 + Math.max(0, taskCount - 1) * 8 : 0;
  const contentHeight = 16 + 28 + (settingsOpen ? 8 + settingsHeight : 0)
    + (hasTasks ? 8 + taskHeight : 0);

  return {
    width,
    height: Math.min(320, Math.max(48, contentHeight)),
  };
}

async function syncOverlayWindowBounds(settingsOpen: boolean, taskCount: number) {
  if (!("__TAURI_INTERNALS__" in window)) {
    return;
  }

  try {
    const { getCurrentWindow, currentMonitor, primaryMonitor, LogicalSize, PhysicalPosition } =
      await import("@tauri-apps/api/window");
    const appWindow = getCurrentWindow();
    const size = desiredWindowSize(settingsOpen, taskCount);
    await appWindow.setSize(new LogicalSize(size.width, size.height));

    const monitor = (await currentMonitor()) ?? (await primaryMonitor());
    if (!monitor) {
      return;
    }

    const scaleFactor = monitor.scaleFactor;
    const x = monitor.workArea.position.x
      + monitor.workArea.size.width
      - Math.round(size.width * scaleFactor)
      - Math.round(16 * scaleFactor);
    const y = monitor.workArea.position.y + Math.round(16 * scaleFactor);

    await appWindow.setPosition(
      new PhysicalPosition(Math.max(monitor.workArea.position.x, x), y),
    );
  } catch (error) {
    console.warn("Failed to sync overlay window bounds", error);
  }
}

function App() {
  const mountedRef = useRef(false);
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
    syncOverlayWindowBounds(settingsOpen, tasks.length);
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

  return (
    <main className={`app-shell${settingsOpen ? " app-shell--settings-open" : ""}`}>
      <button
        className="settings-button"
        type="button"
        onClick={() => setSettingsOpen((isOpen) => !isOpen)}
      >
        Settings
      </button>

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

      <TaskOverlay tasks={tasks} onTaskViewed={handleTaskViewed} />
    </main>
  );
}

export default App;
