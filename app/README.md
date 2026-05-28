# Codex Status Light

Codex Status Light is a small macOS Tauri overlay for watching local Codex work. The app listens for Codex hook events on `http://127.0.0.1:17321/codex-hook`, turns them into task states, and can send one-line Bark notifications to an iPhone when Codex needs permission, confirmation, or reports another important status.

## Development Commands

Run commands from this `app/` directory unless noted otherwise.

```bash
npm install
npm test
npm run build
npm run tauri dev
```

Rust tests can be run from the repository root:

```bash
. "$HOME/.cargo/env" && cargo test --manifest-path app/src-tauri/Cargo.toml
```

## Bark Endpoint

Open the app settings panel and paste a Bark endpoint URL such as:

```text
https://api.day.app/YOUR_DEVICE_KEY
```

Enable notifications in the same panel, then save settings. Settings are persisted at:

```text
~/.codex-status-light/settings.json
```

If the Bark URL is empty or notifications are disabled, hook events still update the overlay but no phone notification is sent.

## Codex Hooks

The app includes a forwarding hook script at:

```text
app/scripts/codex-status-hook.js
```

With the app running, click `Install hooks` in settings. This updates:

```text
~/.codex/hooks.json
```

The installer adds commands for `UserPromptSubmit`, `PermissionRequest`, and `Stop` while preserving existing hook entries. Each installed entry runs the bundled script with Node and forwards Codex hook JSON from stdin to the local Tauri endpoint.

## Manual Hook Verification

Start the desktop app first:

```bash
npm run tauri dev
```

In a second terminal, send these payloads. Each command should print `200`.

### UserPromptSubmit

```bash
curl -s -o /dev/null -w "%{http_code}\n" \
  -X POST http://127.0.0.1:17321/codex-hook \
  -H "content-type: application/json" \
  -d '{"hook_event_name":"UserPromptSubmit","session_id":"manual-s1","prompt":"修复设置面板窄屏遮挡并运行测试"}'
```

Expected state: the overlay shows a Codex task with status `正在执行` (`executing`).

### PermissionRequest

```bash
curl -s -o /dev/null -w "%{http_code}\n" \
  -X POST http://127.0.0.1:17321/codex-hook \
  -H "content-type: application/json" \
  -d '{"hook_event_name":"PermissionRequest","session_id":"manual-s1","tool_name":"exec_command","tool_input":{"cmd":"npm test"}}'
```

Expected state: the same task changes to `需要权限` (`needs_permission`). If Bark notifications are enabled and the endpoint is valid, the phone receives a short permission notification.

### Stop

```bash
curl -s -o /dev/null -w "%{http_code}\n" \
  -X POST http://127.0.0.1:17321/codex-hook \
  -H "content-type: application/json" \
  -d '{"hook_event_name":"Stop","session_id":"manual-s1","last_assistant_message":"已完成修改并通过测试。","stop_hook_active":false}'
```

Expected state: the task changes to `已完成` (`completed`).

To verify the confirmation path, send a `Stop` payload with a confirmation-style assistant message:

```bash
curl -s -o /dev/null -w "%{http_code}\n" \
  -X POST http://127.0.0.1:17321/codex-hook \
  -H "content-type: application/json" \
  -d '{"hook_event_name":"Stop","session_id":"manual-s2","last_assistant_message":"是否继续执行下一步？","stop_hook_active":false}'
```

Expected state: the task changes to `需要确认` (`needs_confirmation`).

When finished, stop `npm run tauri dev` with `Ctrl+C`.
