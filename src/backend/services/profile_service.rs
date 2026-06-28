use crate::backend::error::{AppError, AppResult};
use crate::backend::services::{bepinex_service, core_service, profile_zip_service};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use crate::backend::directories;

const PROFILE_METADATA_FILE: &str = "metadata.json";
const CUSTOM_ICON_BASE_NAME: &str = "icon";
const CUSTOM_ICON_EXTENSIONS: [&str; 7] =
    [".png", ".jpg", ".jpeg", ".webp", ".gif", ".bmp", ".avif"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileModEntry {
    pub mod_id: String,
    pub version: String,
    pub file: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileEntry {
    pub id: String,
    pub name: String,
    pub path: String,
    pub created_at: i64,
    pub last_launched_at: Option<i64>,
    pub bepinex_installed: Option<bool>,
    pub total_play_time: Option<i64>,
    pub icon_mode: Option<String>,
    pub custom_icon_extension: Option<String>,
    pub icon_mod_id: Option<String>,
    pub mods: Vec<ProfileModEntry>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "mode", rename_all = "lowercase")]
pub enum ProfileIconSelection {
    Default,
    Custom {
        bytes: Vec<u8>,
        extension: String,
    },
    Mod {
        #[serde(rename = "modId")]
        mod_id: String,
    },
}

fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| i64::try_from(duration.as_millis()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

fn slugify(input: &str) -> String {
    let mut out = String::new();
    let mut prev_dash = false;

    for ch in input.to_lowercase().chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            prev_dash = false;
        } else if !prev_dash {
            out.push('-');
            prev_dash = true;
        }
    }

    out.trim_matches('-').to_string()
}

fn build_profile_id(name: &str, timestamp: i64) -> String {
    let slug = slugify(name);
    if slug.is_empty() {
        format!("profile-{timestamp}")
    } else {
        format!("{slug}-{timestamp}")
    }
}

fn metadata_path(profile_dir: &Path) -> PathBuf {
    profile_dir.join(PROFILE_METADATA_FILE)
}

fn is_safe_profile_id(id: &str) -> bool {
    let mut components = Path::new(id).components();
    matches!(components.next(), Some(std::path::Component::Normal(_)))
        && components.next().is_none()
}

fn parse_profile(metadata_path: &Path, profile_dir: &Path) -> AppResult<Option<ProfileEntry>> {
    let raw = match fs::read_to_string(metadata_path) {
        Ok(raw) => raw,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let mut profile = serde_json::from_str::<ProfileEntry>(&raw).map_err(|error| {
        AppError::parse(format!(
            "Failed to parse profile metadata at '{}': {error}",
            metadata_path.display()
        ))
    })?;
    profile.path = profile_dir.to_string_lossy().to_string();
    Ok(Some(profile))
}

fn write_profile(profile: &ProfileEntry) -> AppResult<()> {
    let profile_dir = PathBuf::from(&profile.path);
    fs::create_dir_all(&profile_dir)?;
    let metadata = serde_json::to_vec_pretty(profile)?;
    let metadata_path = metadata_path(&profile_dir);
    let temporary_path = metadata_path.with_extension("json.tmp");
    fs::write(&temporary_path, metadata)?;
    fs::rename(&temporary_path, &metadata_path)?;
    Ok(())
}

fn normalize_custom_icon_extension(raw: &str) -> Option<String> {
    let trimmed = raw.trim().to_ascii_lowercase();
    if trimmed.is_empty() {
        return None;
    }
    let normalized = if trimmed.starts_with('.') {
        trimmed
    } else {
        format!(".{trimmed}")
    };
    CUSTOM_ICON_EXTENSIONS
        .contains(&normalized.as_str())
        .then_some(normalized)
}

fn normalize_icon_selection(profile: &mut ProfileEntry) {
    let mode = profile.icon_mode.as_deref().unwrap_or("default");

    match mode {
        "mod" => {
            let has_mod = profile.icon_mod_id.as_ref().is_some_and(|icon_mod_id| {
                profile
                    .mods
                    .iter()
                    .any(|mod_entry| &mod_entry.mod_id == icon_mod_id)
            });
            if !has_mod {
                profile.icon_mode = Some("default".to_string());
                profile.icon_mod_id = None;
            }
        }
        "custom" => {
            if let Some(extension) = profile.custom_icon_extension.as_deref() {
                profile.custom_icon_extension = normalize_custom_icon_extension(extension);
            } else {
                profile.icon_mode = Some("default".to_string());
            }
        }
        _ => {
            profile.icon_mode = Some("default".to_string());
        }
    }

    if profile.icon_mode.as_deref() != Some("mod") {
        profile.icon_mod_id = None;
    }
    if profile.icon_mode.as_deref() != Some("custom") {
        profile.custom_icon_extension = None;
    }
}

fn remove_custom_icon_file(profile: &ProfileEntry, keep_extension: Option<&str>) -> AppResult<()> {
    let Some(extension) = profile
        .custom_icon_extension
        .as_deref()
        .and_then(normalize_custom_icon_extension)
    else {
        return Ok(());
    };

    if keep_extension.is_some_and(|keep| keep == extension) {
        return Ok(());
    }

    let icon_path =
        PathBuf::from(&profile.path).join(format!("{CUSTOM_ICON_BASE_NAME}{extension}"));
    if icon_path.exists() {
        let _ = fs::remove_file(icon_path);
    }
    Ok(())
}

pub fn get_profiles_dir() -> AppResult<String> {
    let dir = directories::app_data_dir()?.join("profiles");
    fs::create_dir_all(&dir)?;
    Ok(dir.to_string_lossy().to_string())
}

pub fn get_profiles() -> AppResult<Vec<ProfileEntry>> {
    let profiles_dir = PathBuf::from(get_profiles_dir()?);
    let mut profiles = Vec::new();

    let entries = match fs::read_dir(&profiles_dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
        Err(error) => return Err(error.into()),
    };

    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                log::warn!("Failed to read profiles directory entry: {error}");
                continue;
            }
        };
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(profile) = parse_profile(&metadata_path(&path), &path)? else {
            continue;
        };
        profiles.push(profile);
    }

    profiles.sort_by(|a, b| {
        let a_launched = a.last_launched_at.unwrap_or(0);
        let b_launched = b.last_launched_at.unwrap_or(0);
        b_launched
            .cmp(&a_launched)
            .then_with(|| b.created_at.cmp(&a.created_at))
    });
    Ok(profiles)
}

pub fn get_profile_by_id(id: &str) -> AppResult<Option<ProfileEntry>> {
    if !is_safe_profile_id(id) {
        return Ok(None);
    }

    let profile_dir = PathBuf::from(get_profiles_dir()?).join(id);
    parse_profile(&metadata_path(&profile_dir), &profile_dir)
}

pub fn create_profile(name: &str) -> AppResult<ProfileEntry> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(AppError::validation("Profile name cannot be empty"));
    }

    let existing = get_profiles()?;
    if existing
        .iter()
        .any(|profile| profile.name.eq_ignore_ascii_case(trimmed))
    {
        return Err(AppError::validation(format!(
            "Profile '{trimmed}' already exists"
        )));
    }

    let timestamp = now_millis();
    let profile_id = build_profile_id(trimmed, timestamp);
    let profile_path = PathBuf::from(get_profiles_dir()?).join(&profile_id);
    fs::create_dir_all(&profile_path)?;

    let profile = ProfileEntry {
        id: profile_id,
        name: trimmed.to_string(),
        path: profile_path.to_string_lossy().to_string(),
        created_at: timestamp,
        last_launched_at: None,
        bepinex_installed: Some(false),
        total_play_time: Some(0),
        icon_mode: Some("default".to_string()),
        custom_icon_extension: None,
        icon_mod_id: None,
        mods: vec![],
    };
    if let Err(error) = write_profile(&profile) {
        let _ = fs::remove_dir_all(&profile_path);
        return Err(error);
    }
    Ok(profile)
}

pub fn install_bepinex_for_profile(profile_id: &str) -> AppResult<()> {
    let mut profile = get_profile_by_id(profile_id)?
        .ok_or_else(|| AppError::validation(format!("Profile '{profile_id}' not found")))?;

    let settings = core_service::get_settings()?;
    let install_arch = match settings.game_platform {
        core_service::GamePlatform::Epic | core_service::GamePlatform::Xbox => "x64",
        core_service::GamePlatform::Steam => "x86",
    };

    let bepinex_url = if install_arch == "x64" {
        settings.bepinex_url_x64.clone()
    } else {
        settings.bepinex_url_x86.clone()
    };

    let cache_path = if settings.cache_bepinex {
        Some(core_service::get_bepinex_cache_path(install_arch)?)
    } else {
        None
    };

    bepinex_service::install_bepinex(
        bepinex_url,
        profile.path.clone(),
        cache_path,
        bepinex_service::BepInExTargetType::Profile,
        profile_id,
    )?;

    profile.bepinex_installed = Some(true);
    write_profile(&profile)?;
    Ok(())
}

pub fn delete_profile(profile_id: &str) -> AppResult<()> {
    let Some(profile) = get_profile_by_id(profile_id)? else {
        return Err(AppError::validation(format!(
            "Profile '{profile_id}' not found"
        )));
    };
    let path = PathBuf::from(profile.path);
    if path.exists() {
        fs::remove_dir_all(path)?;
    }
    Ok(())
}

pub fn rename_profile(profile_id: &str, new_name: &str) -> AppResult<()> {
    let trimmed = new_name.trim();
    if trimmed.is_empty() {
        return Err(AppError::validation("Profile name cannot be empty"));
    }

    let profiles = get_profiles()?;
    if profiles
        .iter()
        .any(|profile| profile.id != profile_id && profile.name.eq_ignore_ascii_case(trimmed))
    {
        return Err(AppError::validation(format!(
            "Profile '{trimmed}' already exists"
        )));
    }

    let Some(mut profile) = get_profile_by_id(profile_id)? else {
        return Err(AppError::validation(format!(
            "Profile '{profile_id}' not found"
        )));
    };
    profile.name = trimmed.to_string();
    write_profile(&profile)
}

pub fn update_profile_icon(profile_id: &str, selection: ProfileIconSelection) -> AppResult<()> {
    let Some(mut profile) = get_profile_by_id(profile_id)? else {
        return Err(AppError::validation(format!(
            "Profile '{profile_id}' not found"
        )));
    };

    match selection {
        ProfileIconSelection::Default => {
            remove_custom_icon_file(&profile, None)?;
            profile.icon_mode = Some("default".to_string());
            profile.custom_icon_extension = None;
            profile.icon_mod_id = None;
            write_profile(&profile)?;
            Ok(())
        }
        ProfileIconSelection::Custom { bytes, extension } => {
            if bytes.is_empty() {
                return Err(AppError::validation("Custom icon image is required"));
            }
            let Some(normalized_extension) = normalize_custom_icon_extension(&extension) else {
                return Err(AppError::validation(
                    "Custom icon must be a PNG, JPG, WEBP, GIF, BMP, or AVIF image",
                ));
            };

            let file_name = format!("{CUSTOM_ICON_BASE_NAME}{normalized_extension}");
            let destination = PathBuf::from(&profile.path).join(file_name);
            fs::write(destination, bytes)?;
            remove_custom_icon_file(&profile, Some(&normalized_extension))?;
            profile.icon_mode = Some("custom".to_string());
            profile.custom_icon_extension = Some(normalized_extension);
            profile.icon_mod_id = None;
            write_profile(&profile)?;
            Ok(())
        }
        ProfileIconSelection::Mod { mod_id } => {
            let normalized_mod_id = mod_id.trim().to_string();
            if normalized_mod_id.is_empty() {
                return Err(AppError::validation("Mod icon selection is required"));
            }
            if !profile
                .mods
                .iter()
                .any(|mod_entry| mod_entry.mod_id == normalized_mod_id)
            {
                return Err(AppError::validation(
                    "Selected mod is not installed in this profile",
                ));
            }

            remove_custom_icon_file(&profile, None)?;
            profile.icon_mode = Some("mod".to_string());
            profile.icon_mod_id = Some(normalized_mod_id);
            profile.custom_icon_extension = None;
            write_profile(&profile)?;
            Ok(())
        }
    }
}

pub fn update_last_launched(profile_id: &str) -> AppResult<()> {
    let Some(mut profile) = get_profile_by_id(profile_id)? else {
        return Ok(());
    };
    profile.last_launched_at = Some(now_millis());
    write_profile(&profile)
}

pub fn add_mod_to_profile(
    profile_id: &str,
    mod_id: &str,
    version: &str,
    file: &str,
) -> AppResult<()> {
    let base_name = Path::new(file)
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| *name == file && !file.contains('/') && !file.contains('\\'))
        .ok_or_else(|| AppError::validation("Invalid mod file name"))?;

    let Some(mut profile) = get_profile_by_id(profile_id)? else {
        return Err(AppError::validation(format!(
            "Profile '{profile_id}' not found"
        )));
    };

    if let Some(existing) = profile
        .mods
        .iter_mut()
        .find(|mod_entry| mod_entry.mod_id == mod_id)
    {
        existing.version = version.to_string();
        existing.file = Some(base_name.to_string());
    } else {
        profile.mods.push(ProfileModEntry {
            mod_id: mod_id.to_string(),
            version: version.to_string(),
            file: Some(base_name.to_string()),
        });
    }
    write_profile(&profile)
}

pub fn add_play_time(profile_id: &str, duration_ms: i64) -> AppResult<()> {
    let Some(mut profile) = get_profile_by_id(profile_id)? else {
        return Err(AppError::validation(format!(
            "Profile '{profile_id}' not found"
        )));
    };
    profile.total_play_time = Some(profile.total_play_time.unwrap_or(0) + duration_ms);
    write_profile(&profile)
}

pub fn remove_mod_from_profile(profile_id: &str, mod_id: &str) -> AppResult<()> {
    let Some(mut profile) = get_profile_by_id(profile_id)? else {
        return Err(AppError::validation(format!(
            "Profile '{profile_id}' not found"
        )));
    };
    profile.mods.retain(|mod_entry| mod_entry.mod_id != mod_id);
    normalize_icon_selection(&mut profile);
    write_profile(&profile)
}

#[allow(dead_code)] // planned: "install mod from local .dll" UI affordance
pub fn import_mod_to_profile(profile_id: &str, source_path: &str) -> AppResult<String> {
    let source = PathBuf::from(source_path);
    if !source.exists() {
        return Err(AppError::validation("Selected mod file does not exist"));
    }

    let source_name = source
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| AppError::validation("Invalid mod file name"))?;

    if !source_name.to_ascii_lowercase().ends_with(".dll") {
        return Err(AppError::validation("Selected file must be a .dll"));
    }

    if source_name.contains('/') || source_name.contains('\\') {
        return Err(AppError::validation("Invalid mod file name"));
    }

    let Some(profile) = get_profile_by_id(profile_id)? else {
        return Err(AppError::validation(format!(
            "Profile '{profile_id}' not found"
        )));
    };

    let destination = PathBuf::from(&profile.path)
        .join("BepInEx")
        .join("plugins")
        .join(source_name);

    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::copy(&source, &destination)?;
    Ok(source_name.to_string())
}

pub fn delete_mod_file(profile_path: &str, file_name: &str) -> AppResult<()> {
    let base_name = Path::new(file_name)
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| AppError::validation("Invalid mod file name"))?;
    if base_name != file_name || file_name.contains('/') || file_name.contains('\\') {
        return Err(AppError::validation("Invalid mod file name"));
    }

    let path = PathBuf::from(profile_path)
        .join("BepInEx")
        .join("plugins")
        .join(base_name);
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

pub fn get_profile_log(profile_path: &str, file_name: &str) -> String {
    let Some(base_name) = Path::new(file_name)
        .file_name()
        .and_then(|name| name.to_str())
    else {
        return String::new();
    };
    if base_name != file_name || file_name.contains('/') || file_name.contains('\\') {
        return String::new();
    }

    let log_path = PathBuf::from(profile_path).join("BepInEx").join(base_name);
    fs::read_to_string(log_path).unwrap_or_default()
}

fn derive_name_from_zip_path(zip_path: &str) -> String {
    let path = PathBuf::from(zip_path);
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .trim()
        .to_string();

    let without_zip = if file_name.to_ascii_lowercase().ends_with(".zip") {
        file_name[..file_name.len() - 4].trim()
    } else {
        file_name.as_str()
    };
    if without_zip.is_empty() {
        "Imported Profile".to_string()
    } else {
        without_zip.to_string()
    }
}

fn make_unique_profile_name(requested: &str, profiles: &[ProfileEntry]) -> String {
    let base = if requested.trim().is_empty() {
        "Imported Profile".to_string()
    } else {
        requested.trim().to_string()
    };
    let existing: HashSet<String> = profiles
        .iter()
        .map(|profile| profile.name.to_lowercase())
        .collect();

    if !existing.contains(&base.to_lowercase()) {
        return base;
    }

    let mut suffix = 2;
    loop {
        let candidate = format!("{base} ({suffix})");
        if !existing.contains(&candidate.to_lowercase()) {
            return candidate;
        }
        suffix += 1;
    }
}

#[derive(Deserialize)]
struct ImportedMetadata {
    name: Option<String>,
    last_launched_at: Option<i64>,
    bepinex_installed: Option<bool>,
    icon_mode: Option<String>,
    custom_icon_extension: Option<String>,
    icon_mod_id: Option<String>,
    mods: Option<serde_json::Value>,
}

#[derive(Clone, Copy, Debug)]
pub enum ZipOp {
    Import,
    Export,
}

/// Progress (0–100) of an in-flight profile import/export, for the UI bar.
#[derive(Clone, Debug)]
pub struct ZipProgress {
    pub op: ZipOp,
    pub progress: f64,
}

fn publish_zip_progress(op: ZipOp, progress: f64) {
    crate::backend::events::publish(crate::backend::events::BackendEvent::ZipProgress(
        ZipProgress { op, progress },
    ));
}

pub fn import_profile_zip(zip_path: &str) -> AppResult<Vec<ProfileEntry>> {
    let mut profiles = get_profiles()?;
    let zip_name = derive_name_from_zip_path(zip_path);

    let zip_infos = profile_zip_service::analyze_profile_zip(zip_path)?;
    if zip_infos.is_empty() {
        return Err(crate::backend::error::AppError::validation(
            "Zip file contains no valid profiles.",
        ));
    }

    let mut imported_profiles = Vec::new();
    let mut created_paths = Vec::new();
    let zip_count = zip_infos.len();
    let profiles_dir = PathBuf::from(get_profiles_dir()?);

    for (index, info) in zip_infos.into_iter().enumerate() {
        let timestamp = now_millis() + index as i64;
        let base_name = info
            .metadata_name
            .clone()
            .or_else(|| info.root_prefix.clone())
            .unwrap_or_else(|| {
                if zip_count > 1 {
                    format!("{} ({})", zip_name, index + 1)
                } else {
                    zip_name.clone()
                }
            });

        let profile_id = build_profile_id(&base_name, timestamp);
        let profile_path = profiles_dir.join(&profile_id);
        fs::create_dir_all(&profile_path)?;
        created_paths.push(profile_path.clone());

        let extract_result = profile_zip_service::extract_profile_from_zip(
            zip_path,
            &profile_path.to_string_lossy(),
            info.root_prefix.as_deref(),
            |p| {
                publish_zip_progress(
                    ZipOp::Import,
                    (index as f64 + p / 100.0) / zip_count as f64 * 100.0,
                )
            },
        );

        if let Err(error) = extract_result {
            for path in &created_paths {
                let _ = fs::remove_dir_all(path);
            }
            return Err(error);
        }

        let metadata_path = metadata_path(&profile_path);
        if !metadata_path.exists()
            && let Some(bytes) = info.metadata_bytes
        {
            let _ = fs::write(&metadata_path, &bytes);
        }

        let imported = fs::read_to_string(&metadata_path)
            .ok()
            .and_then(|raw| serde_json::from_str::<ImportedMetadata>(&raw).ok());

        let requested_name = info
            .metadata_name
            .or_else(|| imported.as_ref().and_then(|item| item.name.clone()))
            .unwrap_or(base_name);

        let unique_name = make_unique_profile_name(&requested_name, &profiles);

        let mut profile = ProfileEntry {
            id: profile_id,
            name: unique_name.clone(),
            path: profile_path.to_string_lossy().to_string(),
            created_at: timestamp,
            last_launched_at: imported.as_ref().and_then(|item| item.last_launched_at),
            bepinex_installed: Some(
                imported
                    .as_ref()
                    .map(|item| item.bepinex_installed.unwrap_or(false))
                    .unwrap_or(true),
            ),
            total_play_time: Some(0),
            icon_mode: imported.as_ref().and_then(|item| item.icon_mode.clone()),
            custom_icon_extension: imported
                .as_ref()
                .and_then(|item| item.custom_icon_extension.clone())
                .and_then(|ext| normalize_custom_icon_extension(&ext)),
            icon_mod_id: imported.as_ref().and_then(|item| item.icon_mod_id.clone()),
            mods: imported
                .and_then(|item| item.mods)
                .map(|mods_value| {
                    let mut entries = Vec::new();
                    if let Some(mods_map) = mods_value.as_object() {
                        for (mod_id, version_val) in mods_map {
                            let version = version_val.as_str().unwrap_or("").to_string();
                            entries.push(ProfileModEntry {
                                mod_id: mod_id.clone(),
                                version,
                                file: None,
                            });
                        }
                    } else if let Some(mods_array) = mods_value.as_array() {
                        for mod_entry in mods_array {
                            if let Ok(entry) =
                                serde_json::from_value::<ProfileModEntry>(mod_entry.clone())
                            {
                                entries.push(entry);
                            }
                        }
                    }
                    entries
                })
                .unwrap_or_default()
                .into_iter()
                .map(|mut mod_entry| {
                    mod_entry.file = mod_entry.file.and_then(|file_name| {
                        Path::new(&file_name)
                            .file_name()
                            .and_then(|name| name.to_str())
                            .filter(|name| {
                                *name == file_name
                                    && !file_name.contains('/')
                                    && !file_name.contains('\\')
                            })
                            .map(|name| name.to_string())
                    });
                    mod_entry
                })
                .collect(),
        };
        normalize_icon_selection(&mut profile);
        if let Err(error) = write_profile(&profile) {
            for path in &created_paths {
                let _ = fs::remove_dir_all(path);
            }
            return Err(error);
        }

        profiles.push(profile.clone());
        imported_profiles.push(profile);
    }
    Ok(imported_profiles)
}

pub fn export_profile_zip(profile_id: &str, destination: &str) -> AppResult<()> {
    let Some(profile) = get_profile_by_id(profile_id)? else {
        return Err(AppError::validation(format!(
            "Profile '{profile_id}' not found"
        )));
    };
    profile_zip_service::export_profile_zip(profile.path, destination.to_string(), |p| {
        publish_zip_progress(ZipOp::Export, p)
    })
}
