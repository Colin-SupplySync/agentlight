# AgentLight

AgentLight 解决的是 AI 编程任务“跑着跑着就忘了看”的问题。它把 Codex 的任务状态放到一个很小的桌面状态灯里，让你不用一直盯着 Codex 窗口。需要处理、需要确认或任务完成时，它可以通过 Bark 通知到 iPhone。

目前主要支持：

- macOS 上的 Codex
- iPhone 上的 Bark 通知

功能：

- 在桌面右上角显示 Codex 任务状态
- 支持正在执行、需要权限、需要确认、已完成、异常中断
- 任务完成或需要处理时可发送 Bark 通知到 iPhone
- 支持多个任务同时显示
- 任务消失后，如果状态再次更新，会重新显示

使用方式：

1. 安装并启动 AgentLight
2. 在 iPhone 安装 Bark
3. 把 Bark URL 填入 AgentLight
4. 安装 Codex hooks
5. 正常使用 Codex，AgentLight 会自动显示状态并发送通知

安装、配置、Bark 接入、hooks 安装和验证，都可以让 Codex 自行完成。

---

AgentLight solves the problem of losing track of long-running AI coding tasks. It keeps Codex task status in a tiny desktop status light, so you do not need to watch the Codex window all the time. When a task needs attention or finishes, it can notify your iPhone through Bark.

Currently supported:

- Codex on macOS
- Bark notifications on iPhone

Features:

- Shows Codex task status on the desktop
- Supports running, permission needed, confirmation needed, completed, and interrupted states
- Sends Bark notifications to iPhone when attention is needed or a task finishes
- Shows multiple tasks at the same time
- If a hidden task receives a new status update, it appears again

How to use:

1. Install and start AgentLight
2. Install Bark on your iPhone
3. Add your Bark URL to AgentLight
4. Install Codex hooks
5. Use Codex normally; AgentLight will show status and send notifications automatically

Codex can complete the setup, Bark connection, hook installation, and verification by itself.
