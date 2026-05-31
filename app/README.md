# AgentLight App

This folder contains the Tauri desktop app for AgentLight.

## Development

```bash
npm install
npm test -- --run
npm run build
npm run tauri dev
```

Rust tests:

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

## Local Data

Settings are stored at:

```text
~/.codex-status-light/settings.json
```

Hook decision logs are stored at:

```text
~/.codex-status-light/events.jsonl
```

The app listens for Codex hook events at:

```text
http://127.0.0.1:17321/codex-hook
```

