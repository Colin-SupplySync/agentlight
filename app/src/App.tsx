import { useCallback, useEffect, useState } from "react";
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
  const [tasks, setTasks] = useState<TaskDto[]>([]);
  const [settings, setSettings] = useState<AppSettings>(defaultSettings);
  const [hookStatus, setHookStatus] = useState<HookStatus>("not_installed");
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [saving, setSaving] = useState(false);
  const [installing, setInstalling] = useState(false);

  const refreshTasks = useCallback(async () => {
    setTasks(await listTasks());
  }, []);

  const refreshSettings = useCallback(async () => {
    const [nextSettings, nextHookStatus] = await Promise.all([
      getSettings(),
      getHookStatus(),
    ]);
    setSettings(nextSettings);
    setHookStatus(nextHookStatus);
  }, []);

  useEffect(() => {
    refreshTasks();
    refreshSettings();

    const intervalId = window.setInterval(() => {
      refreshTasks();
      refreshSettings();
    }, 2500);

    return () => window.clearInterval(intervalId);
  }, [refreshSettings, refreshTasks]);

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
    <main className="app-shell">
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
