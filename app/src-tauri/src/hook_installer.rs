use std::path::Path;

use serde_json::{json, Map, Value};

const HOOK_NAMES: [&str; 3] = ["UserPromptSubmit", "PermissionRequest", "Stop"];
const STATUS_HOOK_MARKER: &str = "codex-status-hook.js";

pub fn install_hooks(hooks_path: &Path, hook_script_path: &Path) -> Result<(), String> {
    if let Some(parent) = hooks_path.parent() {
        std::fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }

    let mut root = std::fs::read_to_string(hooks_path)
        .ok()
        .and_then(|contents| serde_json::from_str::<Value>(&contents).ok())
        .unwrap_or_else(|| Value::Object(Map::new()));

    if !root.is_object() {
        root = Value::Object(Map::new());
    }

    let root_object = root.as_object_mut().expect("root was normalized to object");
    let hooks = root_object
        .entry("hooks")
        .or_insert_with(|| Value::Object(Map::new()));
    if !hooks.is_object() {
        *hooks = Value::Object(Map::new());
    }

    let command = format!("node {}", shell_quote_path(hook_script_path));
    let hooks_object = hooks
        .as_object_mut()
        .expect("hooks was normalized to object");
    for hook_name in HOOK_NAMES {
        let hook_entries = hooks_object
            .entry(hook_name.to_string())
            .or_insert_with(|| Value::Array(Vec::new()));
        if !hook_entries.is_array() {
            *hook_entries = Value::Array(Vec::new());
        }

        let entries = hook_entries
            .as_array_mut()
            .expect("hook entries were normalized to array");
        entries.retain(|entry| {
            entry
                .get("command")
                .and_then(Value::as_str)
                .is_none_or(|existing_command| !existing_command.contains(STATUS_HOOK_MARKER))
        });
        entries.push(json!({ "command": command }));
    }

    let contents = serde_json::to_string_pretty(&root).map_err(|err| err.to_string())?;
    std::fs::write(hooks_path, contents).map_err(|err| err.to_string())
}

fn shell_quote_path(path: &Path) -> String {
    let path = path.to_string_lossy();
    format!("'{}'", path.replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_hooks_json_with_codex_status_hooks() {
        let dir = tempfile::tempdir().unwrap();
        let hooks_path = dir.path().join(".codex").join("hooks.json");
        let hook_script_path = dir.path().join("codex-status-hook.js");

        install_hooks(&hooks_path, &hook_script_path).unwrap();

        let hooks: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&hooks_path).unwrap()).unwrap();
        let command = format!("node {}", shell_quote_path(&hook_script_path));

        for hook_name in ["UserPromptSubmit", "PermissionRequest", "Stop"] {
            assert_eq!(
                hooks["hooks"][hook_name],
                serde_json::json!([{ "command": command }])
            );
        }
    }

    #[test]
    fn preserves_existing_hooks_when_installing() {
        let dir = tempfile::tempdir().unwrap();
        let hooks_path = dir.path().join(".codex").join("hooks.json");
        let hook_script_path = dir.path().join("codex-status-hook.js");
        std::fs::create_dir_all(hooks_path.parent().unwrap()).unwrap();
        std::fs::write(
            &hooks_path,
            serde_json::json!({
                "hooks": {
                    "Stop": [
                        { "command": "node /existing/hook.js" }
                    ]
                }
            })
            .to_string(),
        )
        .unwrap();

        install_hooks(&hooks_path, &hook_script_path).unwrap();

        let hooks: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&hooks_path).unwrap()).unwrap();
        let stop_hooks = hooks["hooks"]["Stop"].as_array().unwrap();

        assert!(stop_hooks
            .iter()
            .any(|hook| hook["command"] == "node /existing/hook.js"));
        assert_eq!(stop_hooks.len(), 2);
    }

    #[test]
    fn quotes_script_paths_with_spaces() {
        let dir = tempfile::tempdir().unwrap();
        let hooks_path = dir.path().join(".codex").join("hooks.json");
        let hook_script_path = dir
            .path()
            .join("folder with spaces")
            .join("codex-status-hook.js");

        install_hooks(&hooks_path, &hook_script_path).unwrap();

        let hooks: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&hooks_path).unwrap()).unwrap();
        let command = hooks["hooks"]["Stop"][0]["command"].as_str().unwrap();

        assert!(command.starts_with("node '"));
        assert!(command.ends_with("codex-status-hook.js'"));
    }
}
