import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { TaskOverlay } from "./TaskOverlay";
import type { TaskDto } from "../types";

const tasks: TaskDto[] = [
  {
    id: "s1",
    title: "重构登录页权限判断",
    provider: "Codex",
    status: "executing",
    viewed: false,
  },
  {
    id: "s2",
    title: "接入支付回调测试",
    provider: "Codex",
    status: "needs_permission",
    viewed: false,
  },
];

describe("TaskOverlay", () => {
  it("renders task titles and provider marker", () => {
    render(<TaskOverlay tasks={tasks} onTaskViewed={() => {}} />);

    expect(screen.getByText("正在执行：重构登录页权限判断")).toBeInTheDocument();
    expect(screen.getByText("需要权限：接入支付回调测试")).toBeInTheDocument();
    expect(screen.getAllByText("Codex")).toHaveLength(2);
  });

  it("renders compact status chips instead of traffic dots", () => {
    render(<TaskOverlay tasks={tasks} onTaskViewed={() => {}} />);

    expect(screen.getByLabelText("状态芯片：正在执行")).toHaveTextContent("RUN");
    expect(screen.getByLabelText("状态芯片：需要权限")).toHaveTextContent("AUTH");
    expect(screen.queryByLabelText("executing")).not.toBeInTheDocument();
  });

  it("calls onTaskViewed when a row is clicked", async () => {
    const onTaskViewed = vi.fn();
    render(<TaskOverlay tasks={tasks} onTaskViewed={onTaskViewed} />);

    await userEvent.click(screen.getByText("需要权限：接入支付回调测试"));

    expect(onTaskViewed).toHaveBeenCalledWith("s2");
  });

  it("renders nothing when there are no tasks", () => {
    const { container } = render(<TaskOverlay tasks={[]} onTaskViewed={() => {}} />);
    expect(container.firstChild).toBeNull();
  });
});
