use std::{
    fs,
    io::Cursor,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::{Manager, Runtime};
use tauri_plugin_fs::FsExt;
use tauri_plugin_store::{Store, StoreExt};
use zip::ZipArchive;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileEntry {
    pub id: String,
    pub name: String,
    pub path: String,
    pub created_at: u64,
}

#[tauri::command]
pub fn create_profile(app: tauri::AppHandle, name: String) -> Result<ProfileEntry, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("Profile name cannot be empty".to_string());
    }

    let profiles_dir = ensure_profiles_dir(&app)?;
    let store = app
        .store("registry.json")
        .map_err(|e| format!("Failed to load store: {}", e))?;

    let mut profiles = load_profiles(&store)?;

    if profiles.iter().any(|profile| profile.name.eq_ignore_ascii_case(trimmed)) {
        return Err(format!("Profile '{}' already exists", trimmed));
    }

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("Failed to obtain system time: {}", e))?
        .as_secs();

    let slug = slugify(trimmed);
    let identifier = if slug.is_empty() {
        format!("profile-{}", timestamp)
    } else {
        format!("{slug}-{timestamp}")
    };

    let profile_dir = profiles_dir.join(&identifier);

    app
        .fs_scope()
        .allow_directory(&profile_dir, true)
        .map_err(|e| format!("Failed to update FS scope: {}", e))?;

    fs::create_dir_all(&profile_dir)
        .map_err(|e| format!("Failed to create profile directory: {}", e))?;

    let profile = ProfileEntry {
        id: identifier.clone(),
        name: trimmed.to_string(),
        path: profile_dir.to_string_lossy().to_string(),
        created_at: timestamp,
    };

    if let Err(err) = download_and_extract_bepinex(&profile_dir) {
        let _ = fs::remove_dir_all(&profile_dir);
        return Err(err);
    }

    profiles.push(profile.clone());
    store.set(
        "profiles",
        serde_json::to_value(&profiles).map_err(|e| format!("Failed to serialize profiles: {}", e))?,
    );

    if should_set_active_profile(&store) {
        store.set("active_profile", json!(profile.id.clone()));
    }

    store
        .save()
        .map_err(|e| format!("Failed to save store: {}", e))?;

    Ok(profile)
}

fn ensure_profiles_dir<R: Runtime>(app: &tauri::AppHandle<R>) -> Result<PathBuf, String> {
    let base_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data dir: {}", e))?;

    let profiles_dir = base_dir.join("profiles");

    app
        .fs_scope()
        .allow_directory(&profiles_dir, true)
        .map_err(|e| format!("Failed to update FS scope: {}", e))?;

    fs::create_dir_all(&profiles_dir)
        .map_err(|e| format!("Failed to create profiles directory: {}", e))?;

    Ok(profiles_dir)
}

fn load_profiles<R: Runtime>(store: &Store<R>) -> Result<Vec<ProfileEntry>, String> {
    match store.get("profiles") {
        Some(Value::Array(array)) => serde_json::from_value(Value::Array(array.clone()))
            .map_err(|e| format!("Failed to parse stored profiles: {}", e)),
        Some(value) => Err(format!("Unexpected profiles format: {}", value)),
        None => Ok(Vec::new()),
    }
}

fn should_set_active_profile<R: Runtime>(store: &Store<R>) -> bool {
    store
        .get("active_profile")
        .map(|value| value.is_null())
        .unwrap_or(true)
}

fn slugify(input: &str) -> String {
    let mut slug = String::new();
    let mut last_hyphen = false;

    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            last_hyphen = false;
        } else if matches!(ch, ' ' | '-' | '_' | '.') {
            if !last_hyphen && !slug.is_empty() {
                slug.push('-');
                last_hyphen = true;
            }
        }
    }

    if slug.ends_with('-') {
        slug.pop();
    }

    slug
}

const BEPINEX_FILE: &str = "BepInEx-Unity.IL2CPP-win-x86-6.0.0-be.738+af0cba7.zip";
const BEPINEX_URL: &str = "https://builds.bepinex.dev/projects/bepinex_be/738/BepInEx-Unity.IL2CPP-win-x86-6.0.0-be.738%2Baf0cba7.zip";

fn download_and_extract_bepinex(profile_dir: &Path) -> Result<(), String> {
    let response = reqwest::blocking::get(BEPINEX_URL)
        .map_err(|e| format!("Failed to download {BEPINEX_FILE}: {e}"))?;

    if !response.status().is_success() {
        return Err(format!(
            "Failed to download {BEPINEX_FILE}: HTTP {}",
            response.status()
        ));
    }

    let bytes = response
        .bytes()
        .map_err(|e| format!("Failed to read {BEPINEX_FILE}: {e}"))?;

    let reader = Cursor::new(bytes);
    let mut archive = ZipArchive::new(reader)
        .map_err(|e| format!("Failed to open {BEPINEX_FILE}: {e}"))?;

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| format!("Failed to read archive entry: {e}"))?;

        let entry_path = entry
            .enclosed_name()
            .ok_or_else(|| "Archive entry contained an invalid path".to_string())?
            .to_path_buf();

        let out_path = profile_dir.join(entry_path);

        if entry.is_dir() {
            fs::create_dir_all(&out_path)
                .map_err(|e| format!("Failed to create directory {}: {e}", out_path.display()))?;
            continue;
        }

        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory {}: {e}", parent.display()))?;
        }

        let mut outfile = fs::File::create(&out_path)
            .map_err(|e| format!("Failed to create file {}: {e}", out_path.display()))?;
        std::io::copy(&mut entry, &mut outfile)
            .map_err(|e| format!("Failed to extract file {}: {e}", out_path.display()))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Some(mode) = entry.unix_mode() {
                let perm = fs::Permissions::from_mode(mode);
                fs::set_permissions(&out_path, perm)
                    .map_err(|e| format!("Failed to set permissions on {}: {e}", out_path.display()))?;
            }
        }
    }

    Ok(())
}
