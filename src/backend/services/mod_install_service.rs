//! High-level "install mod into profile" workflow.
//!
//! Resolves a dependency's semver constraint to a concrete published version,
//! picks the platform/arch-specific download target, then drives the existing
//! `mod_download_service` + `profile_service` plumbing for each mod. On any
//! failure the partial install is rolled back so the profile manifest reflects
//! what's actually on disk.

use std::collections::HashMap;
use std::path::PathBuf;

use semver::{Version, VersionReq};

use crate::backend::api::{
    self, DEFAULT_API_BASE_URL, ModDependency, ModResponse, ModVersion, ModVersionInfo,
    PlatformDownload,
};
use crate::backend::error::{AppError, AppResult};
use crate::backend::services::{
    core_service::{self, GamePlatform},
    mod_download_service, profile_service,
};

#[derive(Debug, Clone)]
pub struct ResolvedDependency {
    pub mod_id: String,
    pub mod_name: String,
    pub resolved_version: String,
    pub dependency_type: String,
    pub version_constraint: String,
}

#[derive(Debug, Clone)]
pub struct InstallModInput {
    pub mod_id: String,
    pub version: String,
}

#[derive(Debug, Clone)]
pub struct InstalledModResult {
    pub mod_id: String,
    pub version: String,
    pub file_name: String,
}

struct DownloadTarget {
    url: String,
    file_name: String,
    checksum: Option<String>,
}

/// Pick the newest published version whose semver satisfies `constraint`.
/// Falls back to the newest version if the constraint can't be parsed.
pub fn resolve_version(constraint: &str, versions_sorted_newest_first: &[ModVersion]) -> Option<String> {
    if versions_sorted_newest_first.is_empty() {
        return None;
    }
    if constraint == "*" {
        return Some(versions_sorted_newest_first[0].version.clone());
    }
    if let Ok(req) = VersionReq::parse(constraint) {
        for item in versions_sorted_newest_first {
            if let Ok(version) = Version::parse(&item.version) {
                if req.matches(&version) {
                    return Some(item.version.clone());
                }
            }
        }
    }
    Some(versions_sorted_newest_first[0].version.clone())
}

pub fn resolve_dependencies(
    dependencies: &[ModDependency],
) -> AppResult<Vec<ResolvedDependency>> {
    let mut resolved = Vec::new();
    for dep in dependencies {
        let Ok(mod_item) = api::fetch_mod(&dep.mod_id) else {
            continue;
        };
        let Ok(mut versions) = api::fetch_mod_versions(&dep.mod_id) else {
            continue;
        };
        versions.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        let Some(version) = resolve_version(&dep.version_constraint, &versions) else {
            continue;
        };
        resolved.push(ResolvedDependency {
            mod_id: dep.mod_id.clone(),
            mod_name: mod_item.name,
            resolved_version: version,
            dependency_type: dep.dependency_type.clone(),
            version_constraint: dep.version_constraint.clone(),
        });
    }
    Ok(resolved)
}

fn absolute_url(path_or_url: &str) -> String {
    if path_or_url.starts_with("http://") || path_or_url.starts_with("https://") {
        return path_or_url.to_string();
    }
    let trimmed_base = DEFAULT_API_BASE_URL.trim_end_matches('/');
    let trimmed_path = path_or_url.trim_start_matches('/');
    format!("{trimmed_base}/{trimmed_path}")
}

fn pick_platform_target(
    platforms: &[PlatformDownload],
    fallback_file_name: Option<&str>,
    fallback_checksum: Option<&str>,
    game_platform: &GamePlatform,
    mod_id: &str,
    version: &str,
) -> Option<DownloadTarget> {
    let arch_fallbacks: &[&str] = match game_platform {
        GamePlatform::Epic => &["x64", "x86"],
        _ => &["x86"],
    };
    for arch in arch_fallbacks {
        let Some(entry) = platforms
            .iter()
            .find(|e| e.platform == "windows" && e.architecture == *arch)
        else {
            continue;
        };
        let url = entry.download_url.clone().unwrap_or_else(|| {
            format!("/api/v3/mods/{mod_id}/versions/{version}/file?platform=windows&arch={arch}")
        });
        let file_name = entry
            .file_name
            .clone()
            .or_else(|| fallback_file_name.map(str::to_string))?;
        return Some(DownloadTarget {
            url: absolute_url(&url),
            file_name,
            checksum: entry.checksum.clone().or_else(|| fallback_checksum.map(str::to_string)),
        });
    }
    None
}

fn resolve_download_target(
    mod_id: &str,
    version: &str,
    version_info: &ModVersionInfo,
    game_platform: &GamePlatform,
) -> AppResult<DownloadTarget> {
    if let Some(platforms) = version_info.platforms.as_ref().filter(|p| !p.is_empty()) {
        if let Some(target) = pick_platform_target(
            platforms,
            version_info.file_name.as_deref(),
            version_info.checksum.as_deref(),
            game_platform,
            mod_id,
            version,
        ) {
            return Ok(target);
        }
    }

    let file_name = version_info.file_name.clone().ok_or_else(|| {
        AppError::validation(format!(
            "Mod '{mod_id}' version '{version}' has no downloadable file_name"
        ))
    })?;
    let url = version_info
        .download_url
        .clone()
        .unwrap_or_else(|| format!("/api/v3/mods/{mod_id}/versions/{version}/file"));
    Ok(DownloadTarget {
        url: absolute_url(&url),
        file_name,
        checksum: version_info.checksum.clone(),
    })
}

fn fetch_mod_meta(mod_id: &str) -> AppResult<ModResponse> {
    api::fetch_mod(mod_id)
}

/// Download each mod into the profile's `BepInEx/plugins/` directory and
/// register it in the profile manifest. Returns the list of installed mods
/// (in input order). Rolls back on failure.
pub fn install_mods_for_profile(
    profile_id: &str,
    mods: &[InstallModInput],
) -> AppResult<Vec<InstalledModResult>> {
    let settings = core_service::get_settings()?;
    let game_platform = settings.game_platform.clone();

    let profile = profile_service::get_profile_by_id(profile_id)?
        .ok_or_else(|| AppError::validation(format!("Profile '{profile_id}' not found")))?;
    let profile_path = profile.path.clone();

    // Snapshot prior entries so we can restore the manifest on rollback.
    let mut previous: HashMap<String, Option<(String, Option<String>)>> = HashMap::new();
    for item in mods {
        let prior = profile
            .mods
            .iter()
            .find(|m| m.mod_id == item.mod_id)
            .map(|m| (m.version.clone(), m.file.clone()));
        previous.insert(item.mod_id.clone(), prior);
    }

    let plugins_dir = PathBuf::from(&profile_path).join("BepInEx").join("plugins");
    std::fs::create_dir_all(&plugins_dir)?;

    let mut downloaded: Vec<InstalledModResult> = Vec::new();
    let mut persisted: Vec<InstalledModResult> = Vec::new();

    for item in mods {
        let info = api::fetch_mod_version_info(&item.mod_id, &item.version)?;
        let _meta = fetch_mod_meta(&item.mod_id)?;
        let target = match resolve_download_target(&item.mod_id, &item.version, &info, &game_platform) {
            Ok(t) => t,
            Err(e) => {
                rollback(&profile_path, profile_id, &downloaded, &persisted, &previous);
                return Err(e);
            }
        };

        let destination = plugins_dir.join(&target.file_name);
        if let Err(e) = mod_download_service::download_mod(
            item.mod_id.clone(),
            target.url,
            destination.to_string_lossy().into_owned(),
            target.checksum.clone(),
        ) {
            rollback(&profile_path, profile_id, &downloaded, &persisted, &previous);
            return Err(e);
        }

        downloaded.push(InstalledModResult {
            mod_id: item.mod_id.clone(),
            version: item.version.clone(),
            file_name: target.file_name.clone(),
        });

        if let Err(e) = profile_service::add_mod_to_profile(
            profile_id,
            &item.mod_id,
            &item.version,
            &target.file_name,
        ) {
            rollback(&profile_path, profile_id, &downloaded, &persisted, &previous);
            return Err(e);
        }
        persisted.push(InstalledModResult {
            mod_id: item.mod_id.clone(),
            version: item.version.clone(),
            file_name: target.file_name.clone(),
        });

        // If the file name changed (e.g. upgrading versions), remove the old file.
        if let Some(Some((_v, Some(old_file)))) = previous.get(&item.mod_id) {
            if old_file != &target.file_name {
                let _ = profile_service::delete_mod_file(&profile_path, old_file);
            }
        }
    }

    Ok(downloaded)
}

fn rollback(
    profile_path: &str,
    profile_id: &str,
    downloaded: &[InstalledModResult],
    persisted: &[InstalledModResult],
    previous: &HashMap<String, Option<(String, Option<String>)>>,
) {
    for item in persisted.iter().rev() {
        if let Some(prior) = previous.get(&item.mod_id) {
            match prior {
                Some((version, Some(file))) => {
                    let _ =
                        profile_service::add_mod_to_profile(profile_id, &item.mod_id, version, file);
                }
                _ => {
                    let _ = profile_service::remove_mod_from_profile(profile_id, &item.mod_id);
                }
            }
        }
    }
    for item in downloaded.iter().rev() {
        let _ = profile_service::delete_mod_file(profile_path, &item.file_name);
    }
}
