//! One-time migration of profiles from the previous (Tauri-based) Starlight,
//! which stored its data under `{roaming app data}/dev.allofus.starlight`.
//! Profile folders are format-compatible — `metadata.json` plus the BepInEx
//! tree — so migrating is moving the directory; extra fields this version
//! added are serde-defaulted on first read.

use std::fs;
use std::path::{Path, PathBuf};

use crate::backend::error::AppResult;
use crate::backend::services::profile_service;

/// The previous app's Tauri identifier, which doubled as its data dir name.
const LEGACY_DIR_NAME: &str = "dev.allofus.starlight";

/// The old app's profiles directory, if present on this machine.
fn legacy_profiles_dir() -> Option<PathBuf> {
    let base = directories::BaseDirs::new()?;
    let dir = base.data_dir().join(LEGACY_DIR_NAME).join("profiles");
    dir.is_dir().then_some(dir)
}

/// Legacy profile directories that could be migrated: they contain a
/// `metadata.json` and no directory of the same name exists here yet.
pub fn detect_legacy_profiles() -> Vec<PathBuf> {
    let Some(legacy_dir) = legacy_profiles_dir() else {
        return Vec::new();
    };
    let current_root = profile_service::get_profiles_dir().map(PathBuf::from).ok();

    let Ok(entries) = fs::read_dir(&legacy_dir) else {
        return Vec::new();
    };
    entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir() && path.join("metadata.json").is_file())
        .filter(|path| {
            let Some(name) = path.file_name() else {
                return false;
            };
            current_root
                .as_ref()
                .is_none_or(|root| !root.join(name).exists())
        })
        .collect()
}

/// Move every migratable legacy profile into this app's profiles directory.
/// Returns how many were migrated. Both roots live under the same app-data
/// volume, so this is normally an instant rename per profile; if a rename
/// fails (e.g. across volumes) the profile is copied and the original left
/// in place.
pub fn migrate_legacy_profiles() -> AppResult<usize> {
    let destination_root = PathBuf::from(profile_service::get_profiles_dir()?);
    let mut migrated = 0;
    for source in detect_legacy_profiles() {
        let Some(name) = source.file_name() else {
            continue;
        };
        let destination = destination_root.join(name);
        if destination.exists() {
            continue;
        }
        if fs::rename(&source, &destination).is_err() {
            copy_dir_all(&source, &destination)?;
        }
        migrated += 1;
    }
    Ok(migrated)
}

fn copy_dir_all(source: &Path, destination: &Path) -> AppResult<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)?.flatten() {
        let path = entry.path();
        let target = destination.join(entry.file_name());
        if path.is_dir() {
            copy_dir_all(&path, &target)?;
        } else {
            fs::copy(&path, &target)?;
        }
    }
    Ok(())
}
