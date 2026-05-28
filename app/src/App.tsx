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
