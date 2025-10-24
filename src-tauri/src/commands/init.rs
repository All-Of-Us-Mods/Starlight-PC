use crate::utils::{finder, game::extract_game_version};
use log::info;
use serde_json::json;
use std::path::{Path, PathBuf};
use tauri::{Manager, Runtime};
use tauri_plugin_store::{Store, StoreExt};

#[tauri::command]
pub async fn init_app(app: tauri::AppHandle) -> Result<String, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data dir: {}", e))?;

    let store = app
        .store("registry.json")
        .map_err(|e| format!("Failed to load store: {}", e))?;

    let amongus_path = resolve_among_us_path(&store);
    let (amongus_path, mut response) =
        initialize_store_if_needed(&store, &data_dir, amongus_path)?;

    let sync_message = sync_game_version(&store, amongus_path.as_deref())?;

    if let Some(msg) = sync_message {
        if response.is_empty() {
            response = msg;
        } else {
            response.push_str(" | ");
            response.push_str(&msg);
        }
    }

    Ok(response)
}

fn resolve_among_us_path<R: Runtime>(store: &Store<R>) -> Option<String> {
    store
        .get("amongus_path")
        .and_then(|value| value.as_str().map(String::from))
}

fn initialize_store_if_needed<R: Runtime>(
    store: &Store<R>,
    data_dir: &Path,
    mut path: Option<String>,
) -> Result<(Option<String>, String), String> {
    if store.get("initialized").is_some() {
        return Ok((path, "Already initialized".to_string()));
    }

    if path.is_none() {
        path = finder::get_among_us_paths()
            .first()
            .map(|p| p.to_string_lossy().to_string());
    }

    store.set("initialized", json!(true));
    store.set("profiles", json!([]));
    store.set("active_profile", json!(null));
    store.set("amongus_path", json!(path.clone()));
    store.set("game_version", json!(null));
    store.set("base_game_setup", json!(false));

    info!("Initialized app at: {}", data_dir.display());
    let response = format!("Initialized. Among Us: {:?}", path);

    store
        .save()
        .map_err(|e| format!("Failed to save store: {}", e))?;

    Ok((path, response))
}

#[tauri::command]
pub fn get_among_us_path_from_store(app: tauri::AppHandle) -> Result<Option<String>, String> {
    let store = app
        .store("registry.json")
        .map_err(|e| format!("Failed to load store: {}", e))?;

    Ok(store
        .get("amongus_path")
        .and_then(|v| v.as_str().map(String::from)))
}

#[tauri::command]
pub fn update_among_us_path(app: tauri::AppHandle, new_path: String) -> Result<(), String> {
    if !PathBuf::from(&new_path).exists() {
        return Err(format!("Path does not exist: {}", new_path));
    }

    let store = app
        .store("registry.json")
        .map_err(|e| format!("Failed to load store: {}", e))?;

    store.set("amongus_path", json!(new_path));
    store.set("game_version", json!(null));
    store
        .save()
        .map_err(|e| format!("Failed to save store: {}", e))?;

    Ok(())
}

fn sync_game_version<R: Runtime>(
    store: &Store<R>,
    amongus_path: Option<&str>,
) -> Result<Option<String>, String> {
    let Some(path_str) = amongus_path else {
        return Ok(None);
    };

    let path = Path::new(path_str);

    if !path.exists() {
        return Ok(Some(format!(
            "Among Us path not found at {}",
            path.display()
        )));
    }

    match extract_game_version(path) {
        Ok(version) => {
            let current_version = store
                .get("game_version")
                .and_then(|value| value.as_str().map(String::from));

            if current_version.as_deref() != Some(version.as_str()) {
                store.set("game_version", json!(version.clone()));
                store
                    .save()
                    .map_err(|e| format!("Failed to save store: {}", e))?;

                Ok(Some(format!("Synced game version: {}", version)))
            } else {
                Ok(None)
            }
        }
        Err(err) => Ok(Some(format!("Failed to read game version: {}", err))),
    }
}
