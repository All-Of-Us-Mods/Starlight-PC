use crate::{finder, utils::game::{copy_dir_recursive, extract_game_version}};
use log::info;
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{Manager, Runtime};
use tauri_plugin_store::{Store, StoreExt};

#[tauri::command]
pub async fn init_app(app: tauri::AppHandle) -> Result<String, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data dir: {}", e))?;

    ensure_app_directories(&data_dir)?;

    let store = app
        .store("registry.json")
        .map_err(|e| format!("Failed to load store: {}", e))?;

    let amongus_path = resolve_among_us_path(&store);
    let (amongus_path, mut store_dirty, mut response) =
        initialize_store_if_needed(&store, &data_dir, amongus_path)?;

    let (sync_dirty, sync_message) =
        sync_base_game_cache(&store, &data_dir, amongus_path.as_deref())?;

    store_dirty |= sync_dirty;

    if store_dirty {
        store
            .save()
            .map_err(|e| format!("Failed to save store: {}", e))?;
    }

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

fn ensure_app_directories(data_dir: &Path) -> Result<(), String> {
    for dir in ["profiles", "global/amongus_base", "global/userdata_base"] {
        fs::create_dir_all(data_dir.join(dir)).map_err(|e| e.to_string())?;
    }

    Ok(())
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
) -> Result<(Option<String>, bool, String), String> {
    if store.get("initialized").is_some() {
        return Ok((path, false, "Already initialized".to_string()));
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
    store.set("base_game_setup", json!(false));

    info!("Initialized app at: {}", data_dir.display());
    let response = format!("Initialized. Among Us: {:?}", path);

    Ok((path, true, response))
}

fn sync_base_game_cache<R: Runtime>(
    store: &Store<R>,
    data_dir: &Path,
    amongus_path: Option<&str>,
) -> Result<(bool, Option<String>), String> {
    let mut store_dirty = false;
    let mut message = None;

    if let Some(path) = amongus_path {
        let source = PathBuf::from(path);

        if source.exists() {
            let version = extract_game_version(source.as_path())?;
            let version_dir = data_dir
                .join("global")
                .join(format!("amongus_base/{}", version));

            if !version_dir.exists() {
                copy_dir_recursive(source.as_path(), version_dir.as_path())?;
                info!(
                    "Cached Among Us version {} at {}",
                    version,
                    version_dir.display()
                );
                message = Some(format!("Cached base game v{}", version));
            }

            if !store
                .get("base_game_setup")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                store.set("base_game_setup", json!(true));
                store_dirty = true;
            }
        } else {
            info!(
                "Stored Among Us path '{}' not found; skipping base game sync",
                path
            );
        }
    }

    Ok((store_dirty, message))
}

#[tauri::command]
pub async fn setup_base_game(app: tauri::AppHandle) -> Result<String, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data dir: {}", e))?;

    let store = app
        .store("registry.json")
        .map_err(|e| format!("Failed to load store: {}", e))?;

    if store
        .get("base_game_setup")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        return Err("Base game already set up".into());
    }

    let amongus_path = store
        .get("amongus_path")
        .and_then(|v| v.as_str().map(String::from))
        .ok_or("Among Us path not found in registry")?;

    let source = PathBuf::from(&amongus_path);
    if !source.exists() {
        return Err(format!("Among Us not found at: {}", amongus_path));
    }

    // Extract game version
    let version = extract_game_version(source.as_path())?;
    info!("Detected Among Us version: {}", version);

    // Create versioned base directory
    let base_dir = data_dir.join("global").join(format!("amongus_base/{}", version));
    copy_dir_recursive(source.as_path(), base_dir.as_path())?;

    store.set("base_game_setup", json!(true));
    store
        .save()
        .map_err(|e| format!("Failed to save store: {}", e))?;

    info!("Base game v{} copied to {}", version, base_dir.display());
    Ok(format!("Base game v{} setup complete", version))
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
    store.set("base_game_setup", json!(false));
    store
        .save()
        .map_err(|e| format!("Failed to save store: {}", e))?;

    Ok(())
}
