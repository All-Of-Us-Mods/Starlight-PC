use crate::backend::error::{AppError, AppResult};
use crate::backend::services::bepinex_runtime::BepInExRuntime;
use crate::backend::services::core_service::AppSettings;
use crate::backend::services::installation_service::GameSetup;
use crate::backend::services::plugins::{self, Plugins};
use crate::backend::services::{bepinex_service, core_service, profile_zip_service};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use crate::backend::directories;

const PROFILE_METADATA_FILE: &str = "metadata.json";
/// Id prefix of custom mods: plugins on disk that no catalog mod claims.
/// Catalog ids never contain `:`.
pub const CUSTOM_MOD_PREFIX: &str = "custom:";
const CUSTOM_ICON_BASE_NAME: &str = "icon";
const CUSTOM_ICON_EXTENSIONS: [&str; 7] =
    [".png", ".jpg", ".jpeg", ".webp", ".gif", ".bmp", ".avif"];

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileModEntry {
    pub mod_id: String,
    pub version: String,
    pub file: Option<String>,
    /// Read from the plugins folder, never stored.
    #[serde(skip, default = "default_true")]
    pub enabled: bool,
}

impl ProfileModEntry {
    pub fn is_custom(&self) -> bool {
        self.mod_id.starts_with(CUSTOM_MOD_PREFIX)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileEntry {
    pub id: String,
    pub name: String,
    pub path: String,
    pub created_at: i64,
    pub last_launched_at: Option<i64>,
    pub total_play_time: Option<i64>,
    pub icon_mode: Option<String>,
    pub custom_icon_extension: Option<String>,
    pub icon_mod_id: Option<String>,
    pub mods: Vec<ProfileModEntry>,
    /// Linked installation to launch; `None` uses the default from Settings.
    #[serde(default)]
    pub installation_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BepInExStatus {
    Ready,
    NotInstalled,
    /// Installed, but for the other architecture than the selected game.
    Incompatible,
    /// The selected linked installation no longer exists.
    MissingInstallation,
}

impl ProfileEntry {
    /// With multiple installations turned off, every profile uses the default.
    pub fn installation<'a>(&self, settings: &'a AppSettings) -> AppResult<&'a GameSetup> {
        let id = self
            .installation_id
            .as_deref()
            .filter(|_| settings.show_installation_controls);
        settings.installation(id)
    }

    /// Read from disk each time, so replaced or removed files are reflected.
    pub fn bepinex_status(&self, settings: &AppSettings) -> BepInExStatus {
        let Ok(game) = self.installation(settings) else {
            return BepInExStatus::MissingInstallation;
        };
        match self.bepinex_runtime().installed_arch() {
            None => BepInExStatus::NotInstalled,
            Some(arch) if arch != game.arch() => BepInExStatus::Incompatible,
            Some(_) => BepInExStatus::Ready,
        }
    }

    /// The profile can't launch until BepInEx is (re)installed.
    pub fn needs_bepinex(&self, settings: &AppSettings) -> bool {
        self.bepinex_status(settings) != BepInExStatus::Ready
    }

    pub fn bepinex_runtime(&self) -> BepInExRuntime<'_> {
        BepInExRuntime::new(Path::new(&self.path))
    }

    pub fn plugins(&self) -> Plugins {
        Plugins::new(Path::new(&self.path))
    }

    /// Catalog mods take their enabled state from disk; plugins no catalog mod
    /// claims become custom mods.
    fn attach_plugins(&mut self) {
        let mut on_disk = self.plugins().scan();
        self.mods.retain(|mod_entry| !mod_entry.is_custom());
        for mod_entry in &mut self.mods {
            if let Some(enabled) = mod_entry
                .file
                .as_ref()
                .and_then(|file| on_disk.remove(file))
            {
                mod_entry.enabled = enabled;
            }
        }
        self.mods
            .extend(on_disk.into_iter().map(|(file, enabled)| ProfileModEntry {
                mod_id: format!("{CUSTOM_MOD_PREFIX}{file}"),
                version: String::new(),
                file: Some(file),
                enabled,
            }));
    }
}

#[derive(Debug, Clone)]
pub enum ProfileIconSelection {
    Default,
    Custom { bytes: Vec<u8>, extension: String },
    Mod { mod_id: String },
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
    // Obsolete runtime state and unknown fields are ignored. Reading a profile
    // must work without write access and must not rewrite another version's data.
    let mut profile = serde_json::from_str::<ProfileEntry>(&raw).map_err(|error| {
        AppError::parse(format!(
            "Failed to parse profile metadata at '{}': {error}",
            metadata_path.display()
        ))
    })?;
    profile.path = profile_dir.to_string_lossy().to_string();
    profile.attach_plugins();
    Ok(Some(profile))
}

fn write_profile(profile: &ProfileEntry) -> AppResult<()> {
    let profile_dir = PathBuf::from(&profile.path);
    fs::create_dir_all(&profile_dir)?;
    let mut stored = profile.clone();
    stored.mods.retain(|mod_entry| !mod_entry.is_custom());
    let metadata = serde_json::to_vec_pretty(&stored)?;
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
        total_play_time: Some(0),
        icon_mode: Some("default".to_string()),
        custom_icon_extension: None,
        icon_mod_id: None,
        mods: vec![],
        installation_id: None,
    };
    if let Err(error) = write_profile(&profile) {
        let _ = fs::remove_dir_all(&profile_path);
        return Err(error);
    }
    Ok(profile)
}

/// The profile with `profile_id`, or a validation error naming it.
fn load_profile(profile_id: &str) -> AppResult<ProfileEntry> {
    get_profile_by_id(profile_id)?
        .ok_or_else(|| AppError::validation(format!("Profile '{profile_id}' not found")))
}

/// Install the BepInEx build the game binary needs, unless the profile
/// already has it. A profile holding the other arch's build is reinstalled in
/// place: plugins and config stay, the arch-specific runtime is replaced.
pub fn install_bepinex_for_profile(profile_id: &str) -> AppResult<()> {
    let profile = load_profile(profile_id)?;

    let settings = core_service::get_settings()?;
    let install_arch = profile.installation(&settings)?.arch();

    let cache_path = if settings.cache_bepinex {
        Some(core_service::get_bepinex_cache_path(install_arch)?)
    } else {
        None
    };

    bepinex_service::ensure_installed(
        &profile.bepinex_runtime(),
        install_arch,
        settings.bepinex_url(install_arch),
        cache_path.as_deref(),
        profile_id,
    )
}

pub fn delete_profile(profile_id: &str) -> AppResult<()> {
    let profile = load_profile(profile_id)?;
    let path = PathBuf::from(profile.path);
    if path.exists() {
        fs::remove_dir_all(path)?;
    }
    Ok(())
}

pub fn set_installation(profile_id: &str, installation_id: Option<String>) -> AppResult<()> {
    core_service::get_settings()?.installation(installation_id.as_deref())?;
    let mut profile = load_profile(profile_id)?;
    profile.installation_id = installation_id;
    write_profile(&profile)
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

    let mut profile = load_profile(profile_id)?;
    profile.name = trimmed.to_string();
    write_profile(&profile)
}

pub fn update_profile_icon(profile_id: &str, selection: ProfileIconSelection) -> AppResult<()> {
    let mut profile = load_profile(profile_id)?;

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
    if !plugins::is_valid_file(file) {
        return Err(AppError::validation("Invalid mod file name"));
    }
    let mut profile = load_profile(profile_id)?;
    let entry = ProfileModEntry {
        mod_id: mod_id.to_string(),
        version: version.to_string(),
        file: Some(file.to_string()),
        enabled: true,
    };
    match profile
        .mods
        .iter_mut()
        .find(|existing| existing.mod_id == mod_id)
    {
        Some(existing) => *existing = entry,
        None => profile.mods.push(entry),
    }
    write_profile(&profile)
}

pub fn set_mod_enabled(profile_id: &str, mod_id: &str, enabled: bool) -> AppResult<()> {
    let profile = load_profile(profile_id)?;
    profile
        .plugins()
        .set_enabled(installed_file(&profile, mod_id)?, enabled)
}

pub fn add_play_time(profile_id: &str, duration_ms: i64) -> AppResult<()> {
    let mut profile = load_profile(profile_id)?;
    profile.total_play_time = Some(profile.total_play_time.unwrap_or(0) + duration_ms);
    write_profile(&profile)
}

pub fn remove_mod_from_profile(profile_id: &str, mod_id: &str) -> AppResult<()> {
    let mut profile = load_profile(profile_id)?;
    profile.mods.retain(|mod_entry| mod_entry.mod_id != mod_id);
    normalize_icon_selection(&mut profile);
    write_profile(&profile)
}

pub fn uninstall_mod_from_profile(profile_id: &str, mod_id: &str) -> AppResult<()> {
    let profile = load_profile(profile_id)?;
    if let Ok(file) = installed_file(&profile, mod_id) {
        profile.plugins().remove(file)?;
    }
    remove_mod_from_profile(profile_id, mod_id)
}

/// Copy a plugin into the profile; it shows up as a custom mod. Returns its
/// file name.
pub fn import_mod_to_profile(profile_id: &str, source_path: &str) -> AppResult<String> {
    load_profile(profile_id)?
        .plugins()
        .import(Path::new(source_path))
}

fn installed_file<'a>(profile: &'a ProfileEntry, mod_id: &str) -> AppResult<&'a str> {
    profile
        .mods
        .iter()
        .find(|mod_entry| mod_entry.mod_id == mod_id)
        .ok_or_else(|| AppError::validation("Mod is not installed in this profile"))?
        .file
        .as_deref()
        .ok_or_else(|| AppError::validation("This mod has no plugin file"))
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
    icon_mode: Option<String>,
    custom_icon_extension: Option<String>,
    icon_mod_id: Option<String>,
    mods: Option<serde_json::Value>,
    /// mod id -> plugin file name, written by our exporter alongside the
    /// id -> version `mods` map. Absent in zips from other sources.
    #[serde(default)]
    mod_files: HashMap<String, String>,
}

/// `mods` is an id -> version map from our exporter, or the stored entry list
/// when the zip is a raw profile folder.
fn imported_mods(metadata: ImportedMetadata) -> Vec<ProfileModEntry> {
    let mut mods: Vec<ProfileModEntry> = match metadata.mods {
        Some(serde_json::Value::Object(map)) => map
            .into_iter()
            .map(|(mod_id, version)| ProfileModEntry {
                file: metadata.mod_files.get(&mod_id).cloned(),
                version: version.as_str().unwrap_or_default().to_string(),
                mod_id,
                enabled: true,
            })
            .collect(),
        Some(serde_json::Value::Array(entries)) => entries
            .into_iter()
            .filter_map(|entry| serde_json::from_value(entry).ok())
            .collect(),
        _ => Vec::new(),
    };
    mods.retain(|mod_entry| !mod_entry.is_custom());
    for mod_entry in &mut mods {
        mod_entry.file = mod_entry
            .file
            .take()
            .filter(|file| plugins::is_valid_file(file));
    }
    mods
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
            total_play_time: Some(0),
            icon_mode: imported.as_ref().and_then(|item| item.icon_mode.clone()),
            custom_icon_extension: imported
                .as_ref()
                .and_then(|item| item.custom_icon_extension.clone())
                .and_then(|ext| normalize_custom_icon_extension(&ext)),
            icon_mod_id: imported.as_ref().and_then(|item| item.icon_mod_id.clone()),
            mods: imported.map(imported_mods).unwrap_or_default(),
            installation_id: None,
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
    let profile = load_profile(profile_id)?;
    profile_zip_service::export_profile_zip(profile.path, destination.to_string(), |p| {
        publish_zip_progress(ZipOp::Export, p)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::test_support::{write_test_pe, write_test_runtime};

    struct TempProfileDir(PathBuf);

    impl TempProfileDir {
        fn new(tag: &str) -> Self {
            let dir =
                std::env::temp_dir().join(format!("starlight-test-{tag}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(dir.join("BepInEx").join("plugins")).unwrap();
            Self(dir)
        }

        fn plugins(&self) -> PathBuf {
            self.0.join("BepInEx").join("plugins")
        }
    }

    impl Drop for TempProfileDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn profile_installation_selection_round_trips_and_defaults_for_old_profiles() {
        let dir = TempProfileDir::new("installation-selection");
        let mut profile = profile_at(&dir, vec![]);
        profile.installation_id = Some("epic-install".into());
        let mut json = serde_json::to_value(&profile).unwrap();
        let restored: ProfileEntry = serde_json::from_value(json.clone()).unwrap();
        assert_eq!(restored.installation_id.as_deref(), Some("epic-install"));
        json.as_object_mut().unwrap().remove("installation_id");
        let legacy: ProfileEntry = serde_json::from_value(json).unwrap();
        assert!(legacy.installation_id.is_none());
    }

    fn profile_at(dir: &TempProfileDir, mods: Vec<ProfileModEntry>) -> ProfileEntry {
        ProfileEntry {
            id: "test".into(),
            name: "Test".into(),
            path: dir.0.to_string_lossy().to_string(),
            created_at: 0,
            last_launched_at: None,
            total_play_time: None,
            icon_mode: None,
            custom_icon_extension: None,
            icon_mod_id: None,
            mods,
            installation_id: None,
        }
    }

    fn tracked(mod_id: &str, file: &str) -> ProfileModEntry {
        ProfileModEntry {
            mod_id: mod_id.into(),
            version: "1.0.0".into(),
            file: Some(file.into()),
            enabled: true,
        }
    }

    #[test]
    fn bepinex_status_follows_runtime_files_and_selected_installation() {
        use crate::backend::services::installation_service::{GAME_EXE_NAME, GameInstallation};
        let dir = TempProfileDir::new("bepinex-status");
        let mut profile = profile_at(&dir, Vec::new());
        let runtime = BepInExRuntime::new(&dir.0);
        let mut settings = AppSettings::default();
        let x64_game = dir.0.join("x64-game");
        write_test_pe(&x64_game.join(GAME_EXE_NAME), 0x8664);
        settings.game.among_us_path = x64_game.to_string_lossy().into_owned();
        assert_eq!(
            profile.bepinex_status(&settings),
            BepInExStatus::NotInstalled
        );

        write_test_runtime(&dir.0, 0x014c);
        assert_eq!(
            profile.bepinex_status(&settings),
            BepInExStatus::Incompatible
        );
        write_test_pe(&runtime.coreclr_path(), 0x8664);
        assert_eq!(profile.bepinex_status(&settings), BepInExStatus::Ready);

        let x86_game = dir.0.join("x86-game");
        write_test_pe(&x86_game.join(GAME_EXE_NAME), 0x014c);
        settings.game_installations.push(GameInstallation {
            id: "x86".into(),
            setup: GameSetup {
                among_us_path: x86_game.to_string_lossy().into_owned(),
                ..Default::default()
            },
        });
        settings.show_installation_controls = true;
        profile.installation_id = Some("x86".into());
        assert_eq!(
            profile.bepinex_status(&settings),
            BepInExStatus::Incompatible
        );
        profile.installation_id = Some("removed".into());
        assert_eq!(
            profile.bepinex_status(&settings),
            BepInExStatus::MissingInstallation
        );
        settings.show_installation_controls = false;
        assert_eq!(profile.bepinex_status(&settings), BepInExStatus::Ready);

        profile.installation_id = None;
        fs::remove_file(runtime.assembly_path()).unwrap();
        assert!(profile.needs_bepinex(&settings));
    }

    #[test]
    fn imported_metadata_ignores_obsolete_runtime_values() {
        let imported: ImportedMetadata = serde_json::from_value(serde_json::json!({
            "name": "Imported",
            "bepinex_installed": {"obsolete": "value"},
            "mods": {"reactor": "2.0.0"}
        }))
        .unwrap();
        assert_eq!(imported.name.as_deref(), Some("Imported"));
        assert_eq!(imported.mods.unwrap()["reactor"], "2.0.0");
    }

    #[test]
    fn reading_legacy_metadata_requires_no_writes_and_preserves_unknown_fields() {
        let dir = TempProfileDir::new("readonly-metadata");
        let mut metadata = serde_json::to_value(profile_at(&dir, vec![])).unwrap();
        metadata["bepinex_installed"] = serde_json::json!("x86");
        metadata["future_field"] = serde_json::json!({"preserve": true});
        let path = metadata_path(&dir.0);
        let bytes = serde_json::to_vec(&metadata).unwrap();
        fs::write(&path, &bytes).unwrap();
        // Any attempt to use the metadata writer fails, regardless of OS/ACLs.
        fs::create_dir(path.with_extension("json.tmp")).unwrap();
        let loaded = parse_profile(&path, &dir.0).unwrap().unwrap();
        assert_eq!(loaded.name, "Test");
        assert_eq!(loaded.bepinex_runtime().installed_arch(), None);
        assert_eq!(fs::read(path).unwrap(), bytes);
    }

    #[test]
    fn plugins_on_disk_become_custom_mods_and_drive_enabled_state() {
        let dir = TempProfileDir::new("attach");
        fs::write(dir.plugins().join("Tracked.dll.disabled"), b"x").unwrap();
        fs::write(dir.plugins().join("Loose.dll"), b"x").unwrap();

        let stale_custom = ProfileModEntry {
            mod_id: format!("{CUSTOM_MOD_PREFIX}Gone.dll"),
            version: String::new(),
            file: Some("Gone.dll".into()),
            enabled: true,
        };
        let mut profile = profile_at(
            &dir,
            vec![tracked("catalog-mod", "Tracked.dll"), stale_custom],
        );
        profile.attach_plugins();

        let mods: Vec<_> = profile
            .mods
            .iter()
            .map(|m| (m.mod_id.as_str(), m.enabled))
            .collect();
        assert_eq!(mods, [("catalog-mod", false), ("custom:Loose.dll", true)]);
    }

    #[test]
    fn custom_mods_are_never_stored() {
        let dir = TempProfileDir::new("store");
        fs::write(dir.plugins().join("Loose.dll"), b"x").unwrap();
        let mut profile = profile_at(&dir, vec![tracked("catalog-mod", "Tracked.dll")]);
        profile.attach_plugins();
        write_profile(&profile).unwrap();

        let stored: serde_json::Value =
            serde_json::from_slice(&fs::read(metadata_path(&dir.0)).unwrap()).unwrap();
        assert_eq!(stored["mods"].as_array().unwrap().len(), 1);
        assert!(stored["mods"][0].get("enabled").is_none());
    }

    #[test]
    fn imported_mods_keep_nested_files_and_drop_unsafe_ones() {
        let metadata: ImportedMetadata = serde_json::from_value(serde_json::json!({
            "mods": {"reactor": "2.0.0", "evil": "1.0.0"},
            "mod_files": {"reactor": "Reactor/Reactor.dll", "evil": "../evil.dll"}
        }))
        .unwrap();
        let mods = imported_mods(metadata);
        let reactor = mods.iter().find(|m| m.mod_id == "reactor").unwrap();
        assert_eq!(reactor.file.as_deref(), Some("Reactor/Reactor.dll"));
        let evil = mods.iter().find(|m| m.mod_id == "evil").unwrap();
        assert!(evil.file.is_none());
    }
}
