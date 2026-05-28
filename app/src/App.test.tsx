// @vitest-environment jsdom

import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import App from "./App";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

describe("App", () => {
  it("renders the scaffold shell", () => {
    render(<App />);

    expect(screen.getByRole("heading", { name: "Welcome to Tauri + React" })).toBeTruthy();
  });
});
