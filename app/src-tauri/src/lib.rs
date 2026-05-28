// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
mod codex_provider;
mod confirmation;
mod domain;
mod dto;
mod hook_installer;
mod hook_server;
mod notifier;
mod settings;
mod task_store;
mod title;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use dto::TaskDto;
#[cfg(test)]
use hook_installer::install_hooks;
use hook_installer::install_user_hooks;
use hook_server::{router as hook_router, SharedBackendState};
use settings::{load_settings, save_settings, AppSettings};
use task_store::TaskStore;
use tauri::Manager;

const HOOK_SERVER_ADDR: &str = "127.0.0.1:17321";
const VIEWED_EXPIRATION: Duration = Duration::from_secs(15);

fn settings_path() -> Result<PathBuf, String> {
    dirs::home_dir()
        .map(|home| home.join(".codex-status-light").join("settings.json"))
        .ok_or_else(|| "could not resolve home directory".to_string())
}

fn default_hook_script_path() -> Result<PathBuf, String> {
    let cwd = std::env::current_dir().map_err(|err| err.to_string())?;
    let candidates = [
        cwd.join("scripts").join("codex-status-hook.js"),
        cwd.join("..").join("scripts").join("codex-status-hook.js"),
        cwd.join("app").join("scripts").join("codex-status-hook.js"),
    ];

    candidates
        .into_iter()
        .find(|path| path.exists())
        .ok_or_else(|| "could not find bundled or development codex-status-hook.js".to_string())
}

fn hook_script_path(app: Option<&tauri::AppHandle>) -> Result<PathBuf, String> {
    if let Some(app) = app {
        if let Ok(resource_dir) = app.path().resource_dir() {
            let resource_path = resource_dir.join("scripts").join("codex-status-hook.js");
            if resource_path.exists() {
                return Ok(resource_path);
            }
        }
    }

    default_hook_script_path()
}

fn list_task_dtos_at(store: &Arc<Mutex<TaskStore>>, now: Instant) -> Vec<TaskDto> {
    let mut store = store.lock().expect("task store lock poisoned");
    store.remove_expired_viewed(now, VIEWED_EXPIRATION);
    store
        .visible_tasks()
        .into_iter()
        .map(TaskDto::from)
        .collect()
}

fn save_settings_to_state(
    settings: AppSettings,
    state: &SharedBackendState,
    settings_path: &Path,
) -> Result<AppSettings, String> {
    save_settings(settings_path, &settings)?;
    *state.settings.lock().expect("settings lock poisoned") = settings.clone();
    Ok(settings)
}

#[cfg(test)]
fn install_hooks_and_mark_installed(
    state: &SharedBackendState,
    settings_path: &Path,
    hooks_path: &Path,
    hook_script_path: &Path,
) -> Result<AppSettings, String> {
    install_hooks(hooks_path, hook_script_path)?;
    let mut settings = state
        .settings
        .lock()
        .expect("settings lock poisoned")
        .clone();
    settings.codex_hooks_installed = true;
    save_settings(settings_path, &settings)?;
    *state.settings.lock().expect("settings lock poisoned") = settings.clone();
    Ok(settings)
}

fn install_user_hooks_and_mark_installed(
    state: &SharedBackendState,
    settings_path: &Path,
    hook_script_path: &Path,
) -> Result<AppSettings, String> {
    install_user_hooks(hook_script_path)?;
    let mut settings = state
        .settings
        .lock()
        .expect("settings lock poisoned")
        .clone();
    settings.codex_hooks_installed = true;
    save_settings(settings_path, &settings)?;
    *state.settings.lock().expect("settings lock poisoned") = settings.clone();
    Ok(settings)
}

#[tauri::command]
fn list_tasks(state: tauri::State<SharedBackendState>) -> Vec<TaskDto> {
    list_task_dtos_at(&state.store, Instant::now())
}

#[tauri::command]
fn mark_task_viewed(task_id: String, state: tauri::State<SharedBackendState>) {
    state
        .store
        .lock()
        .expect("task store lock poisoned")
        .mark_viewed(&task_id, Instant::now());
}

#[tauri::command]
fn get_settings(state: tauri::State<SharedBackendState>) -> AppSettings {
    state
        .settings
        .lock()
        .expect("settings lock poisoned")
        .clone()
}

#[tauri::command]
fn save_app_settings(
    settings: AppSettings,
    state: tauri::State<SharedBackendState>,
) -> Result<AppSettings, String> {
    let path = settings_path()?;
    save_settings_to_state(settings, &state, &path)
}

#[tauri::command]
fn install_codex_hooks(
    app: tauri::AppHandle,
    state: tauri::State<SharedBackendState>,
) -> Result<AppSettings, String> {
    let settings_path = settings_path()?;
    let hook_script_path = hook_script_path(Some(&app))?;
    install_user_hooks_and_mark_installed(&state, &settings_path, &hook_script_path)
}

#[tauri::command]
fn get_hook_status(state: tauri::State<SharedBackendState>) -> String {
    if state
        .settings
        .lock()
        .expect("settings lock poisoned")
        .codex_hooks_installed
    {
        "installed".to_string()
    } else {
        "not_installed".to_string()
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let state = SharedBackendState::new(
        TaskStore::new(),
        load_settings(&settings_path().unwrap_or_else(|_| PathBuf::from("settings.json"))),
    );
    let hook_state = state.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(state)
        .setup(move |_app| {
            tauri::async_runtime::spawn(async move {
                match tokio::net::TcpListener::bind(HOOK_SERVER_ADDR).await {
                    Ok(listener) => {
                        if let Err(err) = axum::serve(listener, hook_router(hook_state)).await {
                            eprintln!("hook server exited with error: {err}");
                        }
                    }
                    Err(err) => {
                        eprintln!("failed to bind hook server on {HOOK_SERVER_ADDR}: {err}");
                    }
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_tasks,
            mark_task_viewed,
            get_settings,
            save_app_settings,
            install_codex_hooks,
            get_hook_status
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::NormalizedEvent;
    use crate::settings::OverlayPosition;

    static CURRENT_DIR_LOCK: Mutex<()> = Mutex::new(());

    fn prompt(session_id: &str) -> NormalizedEvent {
        NormalizedEvent::UserPromptSubmit {
            session_id: session_id.into(),
            prompt: "修复登录页并跑测试".into(),
        }
    }

    #[test]
    fn list_tasks_removes_completed_viewed_tasks_after_fifteen_seconds() {
        let store = Arc::new(Mutex::new(TaskStore::new()));
        let viewed_at = Instant::now();
        {
            let mut store = store.lock().unwrap();
            store.apply_event(prompt("s1"));
            store.apply_event(NormalizedEvent::Completed {
                session_id: "s1".into(),
            });
            store.mark_viewed("s1", viewed_at);
        }

        let before = list_task_dtos_at(&store, viewed_at + Duration::from_secs(14));
        let after = list_task_dtos_at(&store, viewed_at + Duration::from_secs(15));

        assert_eq!(before.len(), 1);
        assert!(after.is_empty());
    }

    #[test]
    fn list_tasks_removes_interrupted_viewed_tasks_after_fifteen_seconds() {
        let store = Arc::new(Mutex::new(TaskStore::new()));
        let viewed_at = Instant::now();
        {
            let mut store = store.lock().unwrap();
            store.apply_event(prompt("s1"));
            store.apply_event(NormalizedEvent::Interrupted {
                session_id: "s1".into(),
                reason: "connection lost".into(),
            });
            store.mark_viewed("s1", viewed_at);
        }

        let tasks = list_task_dtos_at(&store, viewed_at + Duration::from_secs(15));

        assert!(tasks.is_empty());
    }

    #[test]
    fn save_settings_updates_state_and_file() {
        let dir = tempfile::tempdir().unwrap();
        let settings_path = dir.path().join("settings.json");
        let state = SharedBackendState::new(TaskStore::new(), AppSettings::default());
        let settings = AppSettings {
            bark_endpoint_url: "https://api.day.app/key".into(),
            codex_hooks_installed: false,
            notifications_enabled: true,
            overlay_position: OverlayPosition::TopLeft,
            start_at_login: false,
        };

        let saved = save_settings_to_state(settings.clone(), &state, &settings_path).unwrap();

        assert_eq!(saved, settings);
        assert_eq!(
            state.settings.lock().unwrap().bark_endpoint_url,
            "https://api.day.app/key"
        );
        assert_eq!(load_settings(&settings_path), settings);
    }

    #[test]
    fn installing_hooks_marks_settings_installed_and_persists_it() {
        let dir = tempfile::tempdir().unwrap();
        let settings_path = dir.path().join("settings.json");
        let hooks_path = dir.path().join("home").join(".codex").join("hooks.json");
        let hook_script_path = dir.path().join("codex-status-hook.js");
        std::fs::write(&hook_script_path, "console.log('hook');").unwrap();
        let state = SharedBackendState::new(TaskStore::new(), AppSettings::default());

        let settings = install_hooks_and_mark_installed(
            &state,
            &settings_path,
            &hooks_path,
            &hook_script_path,
        )
        .unwrap();

        assert!(settings.codex_hooks_installed);
        assert!(state.settings.lock().unwrap().codex_hooks_installed);
        assert!(load_settings(&settings_path).codex_hooks_installed);
        assert!(hooks_path.exists());
    }

    #[test]
    fn default_hook_script_path_errors_when_no_candidate_exists() {
        let _guard = CURRENT_DIR_LOCK.lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let previous_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let result = default_hook_script_path();

        std::env::set_current_dir(previous_dir).unwrap();
        let err = result.unwrap_err();
        assert!(err.contains("codex-status-hook.js"));
    }

    #[test]
    fn default_hook_script_path_uses_existing_development_candidate() {
        let _guard = CURRENT_DIR_LOCK.lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let scripts_dir = dir.path().join("scripts");
        std::fs::create_dir_all(&scripts_dir).unwrap();
        let expected = scripts_dir.join("codex-status-hook.js");
        std::fs::write(&expected, "console.log('hook');").unwrap();
        let previous_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let resolved = default_hook_script_path().unwrap();

        std::env::set_current_dir(previous_dir).unwrap();
        assert_eq!(
            resolved.canonicalize().unwrap(),
            expected.canonicalize().unwrap()
        );
    }
}
