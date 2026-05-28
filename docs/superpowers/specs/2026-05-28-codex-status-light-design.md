# Codex Status Light Design

## Summary

Build a lightweight macOS desktop status light for Codex tasks. The app shows a small transient task stack near the top-right of the desktop and sends iPhone notifications through Bark. The MVP supports Codex only, but the code should keep provider and notifier boundaries so later versions can add Claude Code, Feishu, Telegram, or other channels without redesigning the product.

The product goal is not to expose every Codex detail. It is to answer one question quickly: does the user need to come back to the computer?

## MVP Scope

Included:

- macOS desktop app built with Tauri.
- A small floating task stack that appears only when there are active or recently changed tasks.
- A minimal settings window for Bark URL, notification toggle, overlay position, and hook install status.
- Codex provider based on Codex hooks.
- Bark notifier for one-line iPhone notifications.
- Five task states: executing, needs permission, needs confirmation, completed, interrupted.
- Automatic task names derived from the user's prompt.
- A 15-second disappearance rule for viewed completed/interrupted tasks.

Excluded from MVP:

- A custom iPhone app.
- Remote approval, prompt input, or session control from the phone.
- Claude Code or other providers.
- Feishu, Telegram, email, or generic webhook notification channels.
- Full log viewing or full Codex transcript browsing.
- Manual task renaming.

## User Experience

The desktop overlay is a compact stack of task rows. It is normally absent. When Codex starts or updates a tracked task, the overlay appears near the top-right of the screen. Multiple simultaneous tasks make the stack grow downward.

Each task row contains:

- Three small traffic-light dots on the left.
- The task name in the center.
- A small provider marker on the right. MVP always displays `Codex`.

Only the current state dot is lit, except for the interrupted state, which flashes red and yellow.

Task rows prioritize the task name. The provider is secondary because the user's real concern is the work being done, not the agent brand.

## Task States

### Executing

Meaning: Codex is actively working and does not need the user.

Desktop: green dot lit.

Phone: no Bark notification.

### Needs Permission

Meaning: Codex is waiting for the user to allow or deny a permission request, such as a shell escalation, file edit approval, or network approval.

Desktop: yellow dot lit. The task remains visible until the permission action is completed.

Phone: immediately send Bark notification.

Notification format:

```text
需要权限：<任务名>
```

### Needs Confirmation

Meaning: Codex is waiting for a user decision that is not a formal permission request, such as confirming a plan, choosing an approach, or deciding whether to continue.

Desktop: yellow dot lit. The task remains visible until the confirmation action is completed.

Phone: immediately send Bark notification.

Notification format:

```text
需要确认：<任务名>
```

### Completed

Meaning: The current user instruction has finished. This may include tests or verification if Codex ran them as part of the task.

Desktop: red dot lit.

Phone: send Bark notification.

Notification format:

```text
已完成：<任务名>
```

After the user views the task, if there is no continued action for the same task within 15 seconds, the row disappears from the desktop overlay.

### Interrupted

Meaning: The task cannot continue normally because Codex failed, disconnected, became unresponsive, or reached a command/tool failure it cannot recover from.

Desktop: red and yellow flash.

Phone: immediately send Bark notification.

Notification format:

```text
异常中断：<任务名>
```

After the user views the task, if there is no continued action for the same task within 15 seconds, the row disappears from the desktop overlay.

## Viewed And Disappearance Rules

Viewing a task means the user clicks the desktop task row or opens the main task detail view for that task.

Continuing a task means the same Codex session transitions back into an active state, such as executing, needs permission, or needs confirmation.

Rules:

- Executing tasks remain visible while executing.
- Needs permission and needs confirmation tasks cannot disappear just because they were viewed. They stay visible until the waiting action is completed.
- Completed tasks can disappear 15 seconds after being viewed if the task is not continued.
- Interrupted tasks can disappear 15 seconds after being viewed if the task is not continued.
- If the same task starts again after disappearing, it reappears as a new visible row.

## Task Naming

MVP derives the task name from the user's prompt.

Default rule:

- Use the prompt's leading intent phrase.
- Remove filler words and overly broad helper phrases when practical.
- Keep the name short enough to fit in one row.
- If the title cannot be confidently shortened, use the first concise segment of the prompt with truncation.

Examples:

- `帮我修复移动端导航遮挡并跑测试` -> `修复移动端导航遮挡`
- `重构登录页权限判断` -> `重构登录页权限判断`
- `接入支付回调测试` -> `接入支付回调测试`

Manual renaming is outside MVP.

## Architecture

Use Tauri for the macOS desktop app. The frontend renders the overlay and minimal settings UI. The backend listens to Codex hook events, maintains task state, and sends Bark notifications.

The app has four main modules:

1. Codex Provider
   - Receives Codex hook events.
   - Converts Codex-specific payloads into normalized task events.
   - MVP provider is Codex only.

2. Task State Engine
   - Owns task identity, task names, current state, notification history, viewed state, and disappearance timers.
   - Converts normalized task events into the five product states.
   - Emits renderable state for the overlay.

3. Desktop Overlay
   - Renders the task stack.
   - Displays three-dot status, task name, and provider marker.
   - Sends viewed events to the state engine when the user clicks a task row.
   - Does not contain business-state logic.

4. Notifier
   - Sends notifications for state changes that require phone alerts.
   - MVP implementation is `BarkNotifier`.
   - Future implementations can include Feishu, Telegram, or generic webhook notifiers.

Data flow:

```text
Codex hooks
  -> Codex Provider
  -> Task State Engine
  -> Desktop Overlay
  -> Bark Notifier
```

## Codex Event Mapping

Use Codex hooks as the primary integration point. The app should not rely on parsing transcript files as a stable API because Codex documentation notes that transcript format is not a stable hook interface.

MVP installs user-level hooks in `~/.codex/hooks.json`, with a settings action that can install, update, or show the hook status. Project-local hook installation is outside MVP. The user-level hook forwards lifecycle payloads to the local Tauri app.

Hook-to-app transport uses a localhost HTTP endpoint exposed by the Tauri backend. The endpoint accepts JSON hook payloads and returns quickly. If the app is not running, the hook script logs locally and exits without blocking Codex.

Initial mapping:

- `UserPromptSubmit`: create or update a task, derive task name from the prompt, set state to executing.
- `PermissionRequest`: set state to needs permission and trigger Bark.
- `Stop`: inspect the latest assistant message and stop context.
  - If the assistant appears to be asking the user for a decision, set state to needs confirmation and trigger Bark.
  - Otherwise set state to completed and trigger Bark.
- Tool failure or provider-detected failure: set state to interrupted and trigger Bark.
- Provider connectivity or process failure: set state to interrupted and trigger Bark.

Needs confirmation detection is conservative in MVP. It can use simple phrase matching in the latest assistant message, with examples such as:

- `请选择`
- `是否继续`
- `需要确认`
- `你希望`
- `要不要`
- `confirm`
- `approve`
- `proceed`

If the detector is unsure, it should prefer completed over needs confirmation to avoid false urgent prompts.

## Bark Notification Behavior

The user configures a Bark endpoint URL in app settings.

The notifier sends one-line notifications:

```text
<状态>：<任务名>
```

The MVP does not include summaries, timestamps, logs, or remote action buttons.

Notification deduplication:

- Do not repeatedly notify for the same task entering the same state.
- Notify again if the same task transitions to a different notifiable state.
- Notify again if a previously disappeared task is started again as a new visible task.

Notifiable states:

- Needs permission.
- Needs confirmation.
- Completed.
- Interrupted.

Executing is not notifiable.

## Configuration

MVP settings:

- Bark endpoint URL.
- Overlay position, default top-right.
- Start at login, optional.
- Notification enabled/disabled, default enabled when Bark is configured.
- Codex hook install/update status.

Future settings:

- Notification provider selection.
- Provider selection beyond Codex.
- Custom disappearance delay.
- Custom state phrases for needs-confirmation detection.

## Error Handling

- If Bark notification fails, keep desktop state intact and show a small non-blocking error indicator in settings or task detail.
- If a Codex hook event cannot be parsed, log it locally and ignore that event rather than crashing the app.
- If the state engine receives an unknown event, log and ignore it.
- If the overlay crashes or reloads, restore current task state from the state engine.
- If the app cannot connect to its local event receiver, hook scripts should fail silently after logging locally so they do not block Codex work.

## Known Limits

- Needs confirmation is detected with conservative phrase matching in MVP, so some prompts that truly need a decision may appear as completed. This is preferable to sending noisy false-positive alerts.
- The app does not attempt to understand every Codex failure. Provider-level interruption should cover clear hook, process, and transport failures first.
- The first release is macOS-only because the desktop overlay behavior and Tauri packaging can be kept focused.

## Testing

Core state engine tests:

- User prompt creates executing task.
- Permission request transitions to needs permission and triggers one notification.
- Stop with completion transitions to completed and triggers notification.
- Stop with confirmation phrase transitions to needs confirmation and triggers notification.
- Interrupted event flashes red/yellow and triggers notification.
- Completed task disappears 15 seconds after viewed when not continued.
- Needs permission/confirmation does not disappear after viewed.
- Notification deduplication prevents repeated same-state notifications.

Provider tests:

- Codex hook payloads are normalized correctly.
- Malformed hook payloads are logged and ignored.

Notifier tests:

- Bark URL is called with the correct one-line content.
- Bark failures do not change task state.

UI tests:

- One task renders as a compact row.
- Multiple tasks stack vertically.
- Provider marker shows `Codex`.
- Long task names truncate cleanly.
- Interrupted state visibly flashes red/yellow.
