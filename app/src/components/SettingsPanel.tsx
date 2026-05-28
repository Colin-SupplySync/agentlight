import { useEffect, useState } from "react";
import type { AppSettings, HookStatus } from "../types";

type Props = {
  settings: AppSettings;
  hookStatus: HookStatus;
  onSaveSettings: (settings: AppSettings) => void | Promise<void>;
  onInstallHooks: () => void | Promise<void>;
  saving?: boolean;
  installing?: boolean;
};

function hookStatusLabel(status: HookStatus) {
  return status === "installed" ? "Hooks installed" : "Hooks not installed";
}

export function SettingsPanel({
  settings,
  hookStatus,
  onSaveSettings,
  onInstallHooks,
  saving = false,
  installing = false,
}: Props) {
  const [draft, setDraft] = useState(settings);
  const [dirty, setDirty] = useState(false);

  useEffect(() => {
    if (!dirty) {
      setDraft(settings);
    }
  }, [dirty, settings]);

  async function handleSave() {
    try {
      await onSaveSettings(draft);
      setDirty(false);
    } catch (error) {
      console.warn("Failed to save settings", error);
    }
  }

  function updateDraft(nextDraft: AppSettings) {
    setDirty(true);
    setDraft(nextDraft);
  }

  return (
    <section className="settings-panel" aria-label="Settings">
      <label className="settings-field">
        <span>Bark URL</span>
        <input
          aria-label="Bark URL"
          value={draft.barkEndpointUrl}
          onChange={(event) =>
            updateDraft({ ...draft, barkEndpointUrl: event.currentTarget.value })
          }
          placeholder="https://api.day.app/key"
        />
      </label>

      <label className="settings-toggle">
        <input
          aria-label="Notifications"
          checked={draft.notificationsEnabled}
          type="checkbox"
          onChange={(event) =>
            updateDraft({ ...draft, notificationsEnabled: event.currentTarget.checked })
          }
        />
        <span>{draft.notificationsEnabled ? "Notifications on" : "Notifications off"}</span>
      </label>

      <div className="settings-status">
        <span>{hookStatusLabel(hookStatus)}</span>
        <button type="button" onClick={onInstallHooks} disabled={installing}>
          {installing ? "Installing" : "Install hooks"}
        </button>
      </div>

      <button
        className="settings-save"
        type="button"
        onClick={handleSave}
        disabled={saving}
      >
        {saving ? "Saving" : "Save"}
      </button>
    </section>
  );
}
