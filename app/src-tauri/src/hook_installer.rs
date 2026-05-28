use std::path::{Path, PathBuf};

use serde_json::{json, Map, Value};

const HOOK_NAMES: [&str; 3] = ["UserPromptSubmit", "PermissionRequest", "Stop"];
const STATUS_HOOK_MARKER: &str = "codex-status-hook.js";

pub fn install_user_hooks(hook_script_path: &Path) -> Result<(), String> {
    let home_dir =
        dirs::home_dir().ok_or_else(|| "could not resolve home directory".to_string())?;
    install_hooks_for_home(&home_dir, hook_script_path)
}

pub fn install_hooks_for_home(home_dir: &Path, hook_script_path: &Path) -> Result<(), String> {
    install_hooks(&codex_hooks_path(home_dir), hook_script_path)
}

pub fn codex_hooks_path(home_dir: &Path) -> PathBuf {
    home_dir.join(".codex").join("hooks.json")
}

pub fn install_hooks(hooks_path: &Path, hook_script_path: &Path) -> Result<(), String> {
    if !hook_script_path.exists() {
        return Err(format!(
            "hook script does not exist: {}",
            hook_script_path.display()
        ));
    }

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

    fn write_hook_script(path: &Path) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, "console.log('hook');").unwrap();
    }

    #[test]
    fn creates_hooks_json_with_codex_status_hooks() {
        let dir = tempfile::tempdir().unwrap();
        let hooks_path = dir.path().join(".codex").join("hooks.json");
        let hook_script_path = dir.path().join("codex-status-hook.js");
        write_hook_script(&hook_script_path);

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
        write_hook_script(&hook_script_path);
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
        write_hook_script(&hook_script_path);

        install_hooks(&hooks_path, &hook_script_path).unwrap();

        let hooks: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&hooks_path).unwrap()).unwrap();
        let command = hooks["hooks"]["Stop"][0]["command"].as_str().unwrap();

        assert!(command.starts_with("node '"));
        assert!(command.ends_with("codex-status-hook.js'"));
    }

    #[test]
    fn resolves_user_hooks_path_under_codex_home() {
        let home_dir = Path::new("/Users/example");

        assert_eq!(
            codex_hooks_path(home_dir),
            PathBuf::from("/Users/example/.codex/hooks.json")
        );
    }

    #[test]
    fn installs_hooks_for_home_using_status_script_command() {
        let dir = tempfile::tempdir().unwrap();
        let home_dir = dir.path().join("home");
        let hook_script_path = dir
            .path()
            .join("app")
            .join("scripts")
            .join(STATUS_HOOK_MARKER);
        write_hook_script(&hook_script_path);

        install_hooks_for_home(&home_dir, &hook_script_path).unwrap();

        let hooks_path = home_dir.join(".codex").join("hooks.json");
        let hooks: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&hooks_path).unwrap()).unwrap();
        let command = hooks["hooks"]["Stop"][0]["command"].as_str().unwrap();

        assert_eq!(hooks_path, codex_hooks_path(&home_dir));
        assert_eq!(
            command,
            format!("node {}", shell_quote_path(&hook_script_path))
        );
    }

    #[test]
    fn refuses_to_install_when_status_script_is_missing() {
        let dir = tempfile::tempdir().unwrap();
        let hooks_path = dir.path().join(".codex").join("hooks.json");
        let hook_script_path = dir.path().join("missing-codex-status-hook.js");

        let err = install_hooks(&hooks_path, &hook_script_path).unwrap_err();

        assert!(err.contains("hook script does not exist"));
        assert!(!hooks_path.exists());
    }
}
