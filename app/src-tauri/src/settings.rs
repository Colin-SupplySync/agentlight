use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OverlayPosition {
    TopRight,
    TopLeft,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AppSettings {
    pub bark_endpoint_url: String,
    pub notifications_enabled: bool,
    pub overlay_position: OverlayPosition,
    pub start_at_login: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            bark_endpoint_url: String::new(),
            notifications_enabled: false,
            overlay_position: OverlayPosition::TopRight,
            start_at_login: false,
        }
    }
}

pub fn load_settings(path: &Path) -> AppSettings {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|contents| serde_json::from_str(&contents).ok())
        .unwrap_or_default()
}

pub fn save_settings(path: &Path, settings: &AppSettings) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }

    let contents = serde_json::to_string_pretty(settings).map_err(|err| err.to_string())?;
    std::fs::write(path, contents).map_err(|err| err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_loads_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let settings_path = dir.path().join("missing").join("settings.json");

        let settings = load_settings(&settings_path);

        assert_eq!(settings, AppSettings::default());
    }

    #[test]
    fn saves_and_loads_settings() {
        let dir = tempfile::tempdir().unwrap();
        let settings_path = dir.path().join("nested").join("settings.json");
        let settings = AppSettings {
            bark_endpoint_url: "https://api.day.app/example".into(),
            notifications_enabled: true,
            overlay_position: OverlayPosition::TopLeft,
            start_at_login: true,
        };

        save_settings(&settings_path, &settings).unwrap();

        assert_eq!(load_settings(&settings_path), settings);
    }
}
