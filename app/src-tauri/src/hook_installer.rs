use std::path::Path;

use serde_json::{json, Map, Value};

const HOOK_NAMES: [&str; 3] = ["UserPromptSubmit", "PermissionRequest", "Stop"];

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

    let command = format!("node {}", hook_script_path.display());
    let hooks_object = hooks
        .as_object_mut()
        .expect("hooks was normalized to object");
    for hook_name in HOOK_NAMES {
        hooks_object.insert(hook_name.to_string(), json!([{ "command": command }]));
    }

    let contents = serde_json::to_string_pretty(&root).map_err(|err| err.to_string())?;
    std::fs::write(hooks_path, contents).map_err(|err| err.to_string())
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
        let command = format!("node {}", hook_script_path.display());

        for hook_name in ["UserPromptSubmit", "PermissionRequest", "Stop"] {
            assert_eq!(
                hooks["hooks"][hook_name],
                serde_json::json!([{ "command": command }])
            );
        }
    }
}
