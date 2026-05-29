// @vitest-environment jsdom

import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import App from "./App";
import {
  getHookStatus,
  getSettings,
  installCodexHooks,
  listTasks,
  markTaskViewed,
  saveAppSettings,
  setOverlayWindowBounds,
} from "./api";

vi.mock("./api", () => ({
  getHookStatus: vi.fn(),
  getSettings: vi.fn(),
  installCodexHooks: vi.fn(),
  listTasks: vi.fn(),
  markTaskViewed: vi.fn(),
  saveAppSettings: vi.fn(),
  setOverlayWindowBounds: vi.fn(),
}));

const defaultSettings = {
  barkEndpointUrl: "",
  codexHooksInstalled: false,
  notificationsEnabled: false,
  overlayPosition: "top_right" as const,
  startAtLogin: false,
};

describe("App", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(getSettings).mockResolvedValue(defaultSettings);
    vi.mocked(getHookStatus).mockResolvedValue("not_installed");
    vi.mocked(listTasks).mockResolvedValue([
      {
        id: "s1",
        title: "修复登录页权限判断",
        provider: "Codex",
        status: "needs_permission",
        viewed: false,
      },
    ]);
    vi.mocked(markTaskViewed).mockResolvedValue(undefined);
    vi.mocked(saveAppSettings).mockResolvedValue(defaultSettings);
    vi.mocked(setOverlayWindowBounds).mockResolvedValue(undefined);
    vi.mocked(installCodexHooks).mockResolvedValue({
      ...defaultSettings,
      codexHooksInstalled: true,
    });
  });

  it("loads settings and renders task overlay", async () => {
    render(<App />);

    expect(await screen.findByText("修复登录页权限判断")).toBeInTheDocument();
    expect(getSettings).toHaveBeenCalled();
    expect(getHookStatus).toHaveBeenCalled();
    expect(listTasks).toHaveBeenCalled();
  });

  it("keeps rendering when task polling rejects", async () => {
    const warnSpy = vi.spyOn(console, "warn").mockImplementation(() => {});
    vi.mocked(listTasks).mockRejectedValueOnce(new Error("task API offline"));

    try {
      render(<App />);

      expect(await screen.findByRole("button", { name: "Settings" })).toBeInTheDocument();
      await waitFor(() => expect(warnSpy).toHaveBeenCalled());
    } finally {
      warnSpy.mockRestore();
    }
  });

  it("marks a clicked task as viewed and refreshes tasks", async () => {
    vi.mocked(listTasks)
      .mockResolvedValueOnce([
        {
          id: "s1",
          title: "修复登录页权限判断",
          provider: "Codex",
          status: "needs_permission",
          viewed: false,
        },
      ])
      .mockResolvedValueOnce([]);

    render(<App />);

    await userEvent.click(await screen.findByText("修复登录页权限判断"));

    expect(markTaskViewed).toHaveBeenCalledWith("s1");
    await waitFor(() => expect(listTasks).toHaveBeenCalledTimes(2));
  });

  it("opens settings and saves the Bark URL", async () => {
    render(<App />);

    await userEvent.click(await screen.findByRole("button", { name: "Settings" }));
    await userEvent.type(screen.getByLabelText("Bark URL"), "https://api.day.app/key");
    await userEvent.click(screen.getByRole("button", { name: "Save" }));

    expect(saveAppSettings).toHaveBeenCalledWith({
      ...defaultSettings,
      barkEndpointUrl: "https://api.day.app/key",
    });
  });

  it("installs hooks from settings", async () => {
    render(<App />);

    await userEvent.click(await screen.findByRole("button", { name: "Settings" }));
    await userEvent.click(screen.getByRole("button", { name: "Install hooks" }));

    expect(installCodexHooks).toHaveBeenCalled();
    await waitFor(() => expect(getHookStatus).toHaveBeenCalledTimes(2));
  });
});
