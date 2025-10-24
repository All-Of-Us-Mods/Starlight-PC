use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use log::{info, warn};
use serde_json::Value;
use tauri::AppHandle;
use tauri_plugin_store::{Store, StoreExt};

use super::profiles::ProfileEntry;
use crate::utils::finder::is_among_us_running;

#[tauri::command]
pub fn launch_among_us(app: AppHandle, profile_id: Option<String>) -> Result<(), String> {
    if is_among_us_running() {
        warn!("Launch aborted because Among Us is already running");
        return Err("Among Us is already running".to_string());
    }

    info!(
        "Launch request received for {}",
        profile_id
            .as_deref()
            .map(|id| format!("profile id '{id}'"))
            .unwrap_or_else(|| "currently active profile".to_string())
    );

    let store = app
        .store("registry.json")
        .map_err(|e| format!("Failed to load store: {e}"))?;

    let among_us_dir = resolve_among_us_dir(&store)?;
    info!("Resolved Among Us directory: {}", among_us_dir.display());
    let exe_path = among_us_dir.join("Among Us.exe");

    if !exe_path.exists() {
        warn!("Launch aborted; Among Us executable missing at {}", exe_path.display());
        return Err(format!("Among Us executable not found at {}", exe_path.display()));
    }

    let profile = resolve_profile(&store, profile_id.as_deref())?;
    let profile_path = PathBuf::from(&profile.path);
    info!(
        "Using profile '{}' ({}) located at {}",
        profile.name,
        profile.id,
        profile_path.display()
    );

    validate_profile_layout(&profile_path)?;
    info!("Profile layout validated for {}", profile_path.display());

    #[cfg(windows)]
    set_dll_directory(&profile_path)?;

    let arguments = build_doorstop_arguments(&profile_path);
    info!("Launching Among Us with arguments: {:?}", arguments);

    Command::new(&exe_path)
        .current_dir(&among_us_dir)
        .args(&arguments)
        .spawn()
        .map_err(|e| format!("Failed to launch Among Us: {e}"))?;

    info!("Among Us launch command spawned successfully");

    Ok(())
}

fn resolve_among_us_dir<R: tauri::Runtime>(store: &Store<R>) -> Result<PathBuf, String> {
    let value = store
        .get("amongus_path")
        .ok_or_else(|| "Among Us path is not configured".to_string())?;

    let raw = value
        .as_str()
        .ok_or_else(|| "Among Us path is not configured".to_string())?;

    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("Among Us path is not configured".to_string());
    }

    let path = PathBuf::from(trimmed);

    if !path.is_dir() {
        return Err(format!("Among Us path does not exist: {}", path.display()));
    }

    Ok(path)
}

fn resolve_profile<R: tauri::Runtime>(store: &Store<R>, profile_id: Option<&str>) -> Result<ProfileEntry, String> {
    let target_id = if let Some(id) = profile_id {
        let trimmed = id.trim();
        if trimmed.is_empty() {
            return Err("Profile id is empty".to_string());
        }
        trimmed.to_string()
    } else {
        let value = store
            .get("active_profile")
            .ok_or_else(|| "No active profile selected".to_string())?;

        let raw = value
            .as_str()
            .ok_or_else(|| "No active profile selected".to_string())?;

        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err("No active profile selected".to_string());
        }

        trimmed.to_string()
    };

    let profiles_value = store
        .get("profiles")
        .unwrap_or(Value::Array(vec![]));

    let profiles: Vec<ProfileEntry> = serde_json::from_value(profiles_value)
        .map_err(|e| format!("Failed to parse profiles: {e}"))?;

    profiles
        .into_iter()
        .find(|profile| profile.id == target_id)
        .ok_or_else(|| format!("Profile {target_id} not found in store"))
}

fn validate_profile_layout(profile_dir: &Path) -> Result<(), String> {
    if !profile_dir.is_dir() {
        return Err(format!("Profile directory does not exist: {}", profile_dir.display()));
    }

    let required_paths = [
        profile_dir.join("BepInEx"),
        profile_dir
            .join("BepInEx")
            .join("core")
            .join("BepInEx.Unity.IL2CPP.dll"),
        profile_dir.join("dotnet").join("coreclr.dll"),
        profile_dir.join("winhttp.dll"),
    ];

    for path in &required_paths {
        if !path.exists() {
            return Err(format!("Profile is missing required file or directory: {}", path.display()));
        }
    }

    Ok(())
}

fn build_doorstop_arguments(profile_dir: &Path) -> Vec<String> {
    let target_assembly = profile_dir
        .join("BepInEx")
        .join("core")
        .join("BepInEx.Unity.IL2CPP.dll");
    let coreclr_dir = profile_dir.join("dotnet");
    let coreclr_path = coreclr_dir.join("coreclr.dll");

    vec![
        "--doorstop-enabled".to_string(),
        "true".to_string(),
        "--doorstop-target-assembly".to_string(),
        target_assembly.display().to_string(),
        "--doorstop-clr-corlib-dir".to_string(),
        coreclr_dir.display().to_string(),
        "--doorstop-clr-runtime-coreclr-path".to_string(),
        coreclr_path.display().to_string(),
    ]
}

#[cfg(windows)]
fn set_dll_directory(path: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;

    let wide: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let success = unsafe { SetDllDirectoryW(wide.as_ptr()) };

    if success == 0 {
        Err(format!(
            "Failed to set DLL directory: {}",
            std::io::Error::last_os_error()
        ))
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
fn set_dll_directory(_path: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(windows)]
#[link(name = "Kernel32")]
extern "system" {
    fn SetDllDirectoryW(lp_path_name: *const u16) -> i32;
}
