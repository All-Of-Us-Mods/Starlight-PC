use crate::backend::error::{AppError, AppResult};
use crate::backend::services::{bepinex_service, core_service, profile_zip_service};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_store::StoreExt;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "source", rename_all = "lowercase")]
pub enum UnifiedMod {
    Managed {
        mod_id: String,
        version: String,
        file: String,
    },
    Custom {
        file: String,
    },
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

#[derive(Debug, Clone, Serialize)]
pub struct ProfileInstallArgs {
    pub profile_id: String,
    pub profile_path: String,
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

fn parse_profile(metadata_path: &Path, profile_dir: &Path) -> Option<ProfileEntry> {
    let raw = fs::read_to_string(metadata_path).ok()?;
    let mut profile = serde_json::from_str::<ProfileEntry>(&raw).ok()?;
    profile.path = profile_dir.to_string_lossy().to_string();
    Some(profile)
}

fn load_legacy_profiles_by_id<R: Runtime>(
    app: &AppHandle<R>,
) -> AppResult<HashMap<String, ProfileEntry>> {
    let store = app
        .store("registry.json")
        .map_err(|e| AppError::state(format!("Failed to load registry store: {e}")))?;

    let Some(raw) = store.get("profiles") else {
        return Ok(HashMap::new());
    };

    let Ok(entries) = serde_json::from_value::<Vec<ProfileEntry>>(raw) else {
        return Ok(HashMap::new());
    };

    Ok(entries
        .into_iter()
        .map(|profile| (profile.id.clone(), profile))
        .collect())
}

fn clear_legacy_profiles_store<R: Runtime>(app: &AppHandle<R>) -> AppResult<()> {
    let store = app
        .store("registry.json")
        .map_err(|e| AppError::state(format!("Failed to load registry store: {e}")))?;

    store.set("profiles", serde_json::json!([]));
    store
        .save()
        .map_err(|e| AppError::state(format!("Failed to save registry store: {e}")))?;
    Ok(())
}

fn write_profile(profile: &ProfileEntry) -> AppResult<()> {
    let profile_dir = PathBuf::from(&profile.path);
    fs::create_dir_all(&profile_dir)?;
    let metadata = serde_json::to_vec_pretty(profile)?;
    fs::write(metadata_path(&profile_dir), metadata)?;
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
    let mode = profile
        .icon_mode
        .clone()
        .unwrap_or_else(|| "default".to_string());

    match mode.as_str() {
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

pub fn get_profiles_dir<R: Runtime>(app: &AppHandle<R>) -> AppResult<String> {
    let dir = app.path().app_data_dir()?.join("profiles");
    fs::create_dir_all(&dir)?;
    Ok(dir.to_string_lossy().to_string())
}

pub fn get_profiles<R: Runtime>(app: &AppHandle<R>) -> AppResult<Vec<ProfileEntry>> {
    let profiles_dir = PathBuf::from(get_profiles_dir(app)?);
    let mut profiles = Vec::new();
    let mut legacy_profiles: Option<HashMap<String, ProfileEntry>> = None;
    let mut migrated_legacy_count = 0usize;

    let entries = match fs::read_dir(&profiles_dir) {
        Ok(entries) => entries,
        Err(_) => return Ok(vec![]),
    };

    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => continue,
        };
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let profile = if let Some(profile) = parse_profile(&metadata_path(&path), &path) {
            profile
        } else {
            let file_name = match path.file_name().and_then(|name| name.to_str()) {
                Some(name) => name.to_string(),
                None => continue,
            };

            if legacy_profiles.is_none() {
                legacy_profiles = Some(load_legacy_profiles_by_id(app)?);
            }

            let Some(mut legacy_profile) = legacy_profiles
                .as_mut()
                .and_then(|profiles_by_id| profiles_by_id.remove(&file_name))
            else {
                continue;
            };

            legacy_profile.path = path.to_string_lossy().to_string();
            normalize_icon_selection(&mut legacy_profile);
            write_profile(&legacy_profile)?;
            migrated_legacy_count += 1;
            legacy_profile
        };
        profiles.push(profile);
    }

    if migrated_legacy_count > 0
        && legacy_profiles
            .as_ref()
            .is_some_and(|profiles_by_id| profiles_by_id.is_empty())
    {
        clear_legacy_profiles_store(app)?;
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

pub fn get_profile_by_id<R: Runtime>(
    app: &AppHandle<R>,
    id: &str,
) -> AppResult<Option<ProfileEntry>> {
    let profile_dir = PathBuf::from(get_profiles_dir(app)?).join(id);
    if let Some(profile) = parse_profile(&metadata_path(&profile_dir), &profile_dir) {
        return Ok(Some(profile));
    }

    if !profile_dir.is_dir() {
        return Ok(None);
    }

    let mut legacy_profiles = load_legacy_profiles_by_id(app)?;
    let Some(mut legacy_profile) = legacy_profiles.remove(id) else {
        return Ok(None);
    };

    legacy_profile.path = profile_dir.to_string_lossy().to_string();
    normalize_icon_selection(&mut legacy_profile);
    write_profile(&legacy_profile)?;

    if legacy_profiles.is_empty() {
        clear_legacy_profiles_store(app)?;
    }

    Ok(Some(legacy_profile))
}

pub fn create_profile<R: Runtime>(app: &AppHandle<R>, name: &str) -> AppResult<ProfileEntry> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(AppError::validation("Profile name cannot be empty"));
    }

    let existing = get_profiles(app)?;
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
    let profile_path = PathBuf::from(get_profiles_dir(app)?).join(&profile_id);
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
    write_profile(&profile)?;
    Ok(profile)
}

pub async fn install_bepinex_for_profile<R: Runtime>(
    app: AppHandle<R>,
    args: ProfileInstallArgs,
) -> AppResult<()> {
    let settings = core_service::get_settings(&app)?;
    let cache_path = if settings.cache_bepinex {
        Some(core_service::get_bepinex_cache_path(&app)?)
    } else {
        None
    };

    bepinex_service::install_bepinex(
        app.clone(),
        settings.bepinex_url,
        args.profile_path.clone(),
        cache_path,
    )
    .await?;

    if let Some(mut profile) = get_profile_by_id(&app, &args.profile_id)? {
        profile.bepinex_installed = Some(true);
        write_profile(&profile)?;
    }
    Ok(())
}

pub fn delete_profile<R: Runtime>(app: &AppHandle<R>, profile_id: &str) -> AppResult<()> {
    let Some(profile) = get_profile_by_id(app, profile_id)? else {
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

pub fn rename_profile<R: Runtime>(
    app: &AppHandle<R>,
    profile_id: &str,
    new_name: &str,
) -> AppResult<()> {
    let trimmed = new_name.trim();
    if trimmed.is_empty() {
        return Err(AppError::validation("Profile name cannot be empty"));
    }

    let profiles = get_profiles(app)?;
    if profiles
        .iter()
        .any(|profile| profile.id != profile_id && profile.name.eq_ignore_ascii_case(trimmed))
    {
        return Err(AppError::validation(format!(
            "Profile '{trimmed}' already exists"
        )));
    }

    let Some(mut profile) = get_profile_by_id(app, profile_id)? else {
        return Err(AppError::validation(format!(
            "Profile '{profile_id}' not found"
        )));
    };
    profile.name = trimmed.to_string();
    write_profile(&profile)
}

pub fn update_profile_icon<R: Runtime>(
    app: &AppHandle<R>,
    profile_id: &str,
    selection: ProfileIconSelection,
) -> AppResult<()> {
    let Some(mut profile) = get_profile_by_id(app, profile_id)? else {
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

pub fn update_last_launched<R: Runtime>(app: &AppHandle<R>, profile_id: &str) -> AppResult<()> {
    let Some(mut profile) = get_profile_by_id(app, profile_id)? else {
        return Ok(());
    };
    profile.last_launched_at = Some(now_millis());
    write_profile(&profile)
}

pub fn add_mod_to_profile<R: Runtime>(
    app: &AppHandle<R>,
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

    let Some(mut profile) = get_profile_by_id(app, profile_id)? else {
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

pub fn add_play_time<R: Runtime>(
    app: &AppHandle<R>,
    profile_id: &str,
    duration_ms: i64,
) -> AppResult<()> {
    let Some(mut profile) = get_profile_by_id(app, profile_id)? else {
        return Err(AppError::validation(format!(
            "Profile '{profile_id}' not found"
        )));
    };
    profile.total_play_time = Some(profile.total_play_time.unwrap_or(0) + duration_ms);
    write_profile(&profile)
}

pub fn remove_mod_from_profile<R: Runtime>(
    app: &AppHandle<R>,
    profile_id: &str,
    mod_id: &str,
) -> AppResult<()> {
    let Some(mut profile) = get_profile_by_id(app, profile_id)? else {
        return Err(AppError::validation(format!(
            "Profile '{profile_id}' not found"
        )));
    };
    profile.mods.retain(|mod_entry| mod_entry.mod_id != mod_id);
    normalize_icon_selection(&mut profile);
    write_profile(&profile)
}

pub fn get_mod_files(profile_path: &str) -> Vec<String> {
    let plugins_dir = PathBuf::from(profile_path).join("BepInEx").join("plugins");
    let Ok(entries) = fs::read_dir(plugins_dir) else {
        return vec![];
    };

    entries
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let file_type = entry.file_type().ok()?;
            if file_type.is_dir() {
                return None;
            }
            let file_name = entry.file_name();
            let file_name = file_name.to_string_lossy().to_string();
            file_name
                .to_ascii_lowercase()
                .ends_with(".dll")
                .then_some(file_name)
        })
        .collect()
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

pub fn read_binary_file<R: Runtime>(app: &AppHandle<R>, path: &str) -> AppResult<Vec<u8>> {
    let allowed_root = PathBuf::from(get_profiles_dir(app)?).canonicalize()?;
    let canonical = PathBuf::from(path)
        .canonicalize()
        .map_err(|_| AppError::validation("Path does not exist"))?;
    if !canonical.starts_with(&allowed_root) {
        return Err(AppError::validation(
            "Path is outside the allowed directory",
        ));
    }
    Ok(fs::read(canonical)?)
}

pub fn delete_unified_mod<R: Runtime>(
    app: &AppHandle<R>,
    profile_id: &str,
    mod_entry: UnifiedMod,
) -> AppResult<()> {
    let Some(profile) = get_profile_by_id(app, profile_id)? else {
        return Err(AppError::validation(format!(
            "Profile '{profile_id}' not found"
        )));
    };

    match mod_entry {
        UnifiedMod::Managed {
            mod_id,
            file,
            version: _,
        } => {
            delete_mod_file(&profile.path, &file)?;
            remove_mod_from_profile(app, profile_id, &mod_id)?;
        }
        UnifiedMod::Custom { file } => {
            delete_mod_file(&profile.path, &file)?;
        }
    }
    Ok(())
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
    icon_mode: Option<String>,
    custom_icon_extension: Option<String>,
    icon_mod_id: Option<String>,
    mods: Option<Vec<ProfileModEntry>>,
}

pub fn import_profile_zip<R: Runtime>(
    app: &AppHandle<R>,
    zip_path: &str,
) -> AppResult<ProfileEntry> {
    let profiles = get_profiles(app)?;
    let zip_name = derive_name_from_zip_path(zip_path);
    let timestamp = now_millis();
    let profile_id = build_profile_id(&zip_name, timestamp);
    let profile_path = PathBuf::from(get_profiles_dir(app)?).join(&profile_id);
    fs::create_dir_all(&profile_path)?;

    let import_result = profile_zip_service::import_profile_zip(
        zip_path.to_string(),
        profile_path.to_string_lossy().to_string(),
    );

    if let Err(error) = import_result {
        let _ = fs::remove_dir_all(&profile_path);
        return Err(error);
    }
    let import_result = import_result?;

    let metadata_path = metadata_path(&profile_path);
    let imported = fs::read_to_string(&metadata_path)
        .ok()
        .and_then(|raw| serde_json::from_str::<ImportedMetadata>(&raw).ok());

    let requested_name = import_result
        .metadata_name
        .or_else(|| imported.as_ref().and_then(|item| item.name.clone()))
        .unwrap_or(zip_name);
    let unique_name = make_unique_profile_name(&requested_name, &profiles);

    let mut profile = ProfileEntry {
        id: profile_id,
        name: unique_name,
        path: profile_path.to_string_lossy().to_string(),
        created_at: timestamp,
        last_launched_at: imported.as_ref().and_then(|item| item.last_launched_at),
        bepinex_installed: Some(true),
        total_play_time: Some(0),
        icon_mode: imported.as_ref().and_then(|item| item.icon_mode.clone()),
        custom_icon_extension: imported
            .as_ref()
            .and_then(|item| item.custom_icon_extension.clone())
            .and_then(|ext| normalize_custom_icon_extension(&ext)),
        icon_mod_id: imported.as_ref().and_then(|item| item.icon_mod_id.clone()),
        mods: imported
            .and_then(|item| item.mods)
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
    write_profile(&profile)?;
    Ok(profile)
}

pub fn export_profile_zip<R: Runtime>(
    app: &AppHandle<R>,
    profile_id: &str,
    destination: &str,
) -> AppResult<()> {
    let Some(profile) = get_profile_by_id(app, profile_id)? else {
        return Err(AppError::validation(format!(
            "Profile '{profile_id}' not found"
        )));
    };
    profile_zip_service::export_profile_zip(profile.path, destination.to_string())
}
