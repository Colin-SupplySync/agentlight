import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { SettingsPanel } from "./SettingsPanel";
import type { AppSettings } from "../types";

const settings: AppSettings = {
  barkEndpointUrl: "",
  codexHooksInstalled: false,
  notificationsEnabled: false,
  overlayPosition: "top_right",
  startAtLogin: false,
};

describe("SettingsPanel", () => {
  it("shows notification and hook status", () => {
    render(
      <SettingsPanel
        hookStatus="not_installed"
        settings={settings}
        onInstallHooks={() => {}}
        onSaveSettings={() => {}}
      />,
    );

    expect(screen.getByLabelText("Bark URL")).toBeInTheDocument();
    expect(screen.getByText("Notifications off")).toBeInTheDocument();
    expect(screen.getByText("Hooks not installed")).toBeInTheDocument();
  });

  it("saves Bark URL and notification preference", async () => {
    const onSaveSettings = vi.fn();
    render(
      <SettingsPanel
        hookStatus="not_installed"
        settings={settings}
        onInstallHooks={() => {}}
        onSaveSettings={onSaveSettings}
      />,
    );

    await userEvent.type(screen.getByLabelText("Bark URL"), "https://api.day.app/key");
    await userEvent.click(screen.getByLabelText("Notifications"));
    await userEvent.click(screen.getByRole("button", { name: "Save" }));

    expect(onSaveSettings).toHaveBeenCalledWith({
      ...settings,
      barkEndpointUrl: "https://api.day.app/key",
      notificationsEnabled: true,
    });
  });

  it("does not overwrite an unsaved Bark URL draft when settings props refresh", async () => {
    const { rerender } = render(
      <SettingsPanel
        hookStatus="not_installed"
        settings={settings}
        onInstallHooks={() => {}}
        onSaveSettings={() => {}}
      />,
    );

    await userEvent.type(screen.getByLabelText("Bark URL"), "https://draft.example/key");

    rerender(
      <SettingsPanel
        hookStatus="installed"
        settings={{
          ...settings,
          barkEndpointUrl: "https://background.example/key",
          notificationsEnabled: true,
        }}
        onInstallHooks={() => {}}
        onSaveSettings={() => {}}
      />,
    );

    expect(screen.getByLabelText("Bark URL")).toHaveValue("https://draft.example/key");
  });

  it("installs hooks", async () => {
    const onInstallHooks = vi.fn();
    render(
      <SettingsPanel
        hookStatus="not_installed"
        settings={settings}
        onInstallHooks={onInstallHooks}
        onSaveSettings={() => {}}
      />,
    );

    await userEvent.click(screen.getByRole("button", { name: "Install hooks" }));

    expect(onInstallHooks).toHaveBeenCalled();
  });
});
