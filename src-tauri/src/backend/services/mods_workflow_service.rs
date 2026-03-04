use crate::backend::error::{AppError, AppResult};
use crate::backend::services::{core_service, mod_download_service, profile_service};
use reqwest::Client;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tauri::{AppHandle, Runtime};

#[derive(Debug, Clone, Deserialize)]
pub struct ModDependency {
    pub mod_id: String,
    pub version_constraint: String,
    pub r#type: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResolvedDependency {
    pub mod_id: String,
    #[serde(rename = "modName")]
    pub mod_name: String,
    #[serde(rename = "resolvedVersion")]
    pub resolved_version: String,
    pub r#type: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallModInput {
    pub mod_id: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct InstalledModResult {
    pub mod_id: String,
    pub version: String,
    pub file_name: String,
}

#[derive(Debug, Clone, Deserialize)]
struct ModResponse {
    name: String,
}

#[derive(Debug, Clone, Deserialize)]
struct ModVersion {
    version: String,
    created_at: i64,
}

#[derive(Debug, Clone, Deserialize)]
struct PlatformDownload {
    platform: String,
    architecture: String,
    file_name: Option<String>,
    checksum: Option<String>,
    download_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct ModVersionInfo {
    file_name: String,
    checksum: String,
    download_url: Option<String>,
    platforms: Option<Vec<PlatformDownload>>,
}

#[derive(Debug, Clone)]
struct DownloadTarget {
    url: String,
    file_name: String,
    checksum: String,
}

fn resolve_absolute_url(base_url: &str, path_or_url: &str) -> String {
    if path_or_url.starts_with("http://") || path_or_url.starts_with("https://") {
        return path_or_url.to_string();
    }

    let trimmed_base = base_url.trim_end_matches('/');
    let trimmed_path = path_or_url.trim_start_matches('/');
    format!("{trimmed_base}/{trimmed_path}")
}

async fn fetch_json<T: for<'de> Deserialize<'de>>(client: &Client, url: &str) -> AppResult<T> {
    let response = client.get(url).send().await.map_err(AppError::from)?;
    if !response.status().is_success() {
        return Err(AppError::other(format!(
            "HTTP error {} while fetching {url}",
            response.status()
        )));
    }
    response.json::<T>().await.map_err(AppError::from)
}

async fn get_mod_by_id(client: &Client, api_base_url: &str, mod_id: &str) -> AppResult<ModResponse> {
    fetch_json(client, &format!("{api_base_url}/api/v2/mods/{mod_id}")).await
}

async fn get_mod_versions(client: &Client, api_base_url: &str, mod_id: &str) -> AppResult<Vec<ModVersion>> {
    fetch_json(client, &format!("{api_base_url}/api/v2/mods/{mod_id}/versions")).await
}

async fn get_mod_version_info(
    client: &Client,
    api_base_url: &str,
    mod_id: &str,
    version: &str,
) -> AppResult<ModVersionInfo> {
    fetch_json(
        client,
        &format!("{api_base_url}/api/v2/mods/{mod_id}/versions/{version}/info"),
    )
    .await
}

fn resolve_version(
    version_constraint: &str,
    versions_sorted: &[ModVersion],
) -> Option<String> {
    if versions_sorted.is_empty() {
        return None;
    }
    if version_constraint == "*" {
        return Some(versions_sorted[0].version.clone());
    }

    let req = semver::VersionReq::parse(version_constraint).ok();
    if let Some(req) = req {
        for item in versions_sorted {
            if let Ok(version) = Version::parse(&item.version)
                && req.matches(&version)
            {
                return Some(item.version.clone());
            }
        }
    }

    Some(versions_sorted[0].version.clone())
}

pub async fn resolve_dependencies(dependencies: Vec<ModDependency>) -> AppResult<Vec<ResolvedDependency>> {
    let api_base_url = core_service::api_base_url();
    let client = Client::new();
    let mut resolved = Vec::new();

    for dependency in dependencies {
        let mod_item = get_mod_by_id(&client, &api_base_url, &dependency.mod_id).await?;
        let mut versions = get_mod_versions(&client, &api_base_url, &dependency.mod_id).await?;

        versions.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        let Some(resolved_version) = resolve_version(&dependency.version_constraint, &versions) else {
            continue;
        };

        resolved.push(ResolvedDependency {
            mod_id: dependency.mod_id,
            mod_name: mod_item.name,
            resolved_version,
            r#type: dependency.r#type,
        });
    }

    Ok(resolved)
}

fn resolve_download_target(
    api_base_url: &str,
    mod_id: &str,
    version: &str,
    version_info: &ModVersionInfo,
    game_platform: core_service::GamePlatform,
) -> DownloadTarget {
    let legacy_path = format!("/api/v2/mods/{mod_id}/versions/{version}/file");
    let default_url = version_info
        .download_url
        .as_deref()
        .unwrap_or(&legacy_path);
    let default_target = DownloadTarget {
        url: resolve_absolute_url(api_base_url, default_url),
        file_name: version_info.file_name.clone(),
        checksum: version_info.checksum.clone(),
    };

    let Some(platforms) = version_info.platforms.as_ref() else {
        return default_target;
    };
    if platforms.is_empty() {
        return default_target;
    }

    let architecture_fallbacks: &[&str] = match game_platform {
        core_service::GamePlatform::Epic => &["x64", "x86"],
        _ => &["x86"],
    };

    for architecture in architecture_fallbacks {
        if let Some(entry) = platforms
            .iter()
            .find(|entry| entry.platform == "windows" && entry.architecture == *architecture)
        {
            let url = entry
                .download_url
                .as_deref()
                .map(str::to_string)
                .unwrap_or_else(|| format!("{legacy_path}?platform=windows&arch={architecture}"));
            return DownloadTarget {
                url: resolve_absolute_url(api_base_url, &url),
                file_name: entry.file_name.clone().unwrap_or_else(|| version_info.file_name.clone()),
                checksum: entry
                    .checksum
                    .clone()
                    .unwrap_or_else(|| version_info.checksum.clone()),
            };
        }
    }

    default_target
}

pub async fn install_mods_for_profile<R: Runtime>(
    app: AppHandle<R>,
    profile_id: &str,
    profile_path: &str,
    mods: Vec<InstallModInput>,
) -> AppResult<Vec<InstalledModResult>> {
    let api_base_url = core_service::api_base_url();
    let settings = core_service::get_settings(&app)?;
    let game_platform = settings.game_platform;
    let client = Client::new();

    let profile = profile_service::get_profile_by_id(&app, profile_id)?
        .ok_or_else(|| AppError::validation(format!("Profile '{profile_id}' not found")))?;

    let mut previous_by_mod_id: HashMap<String, Option<(String, Option<String>)>> = HashMap::new();
    for item in &mods {
        let previous = profile
            .mods
            .iter()
            .find(|mod_entry| mod_entry.mod_id == item.mod_id)
            .map(|mod_entry| (mod_entry.version.clone(), mod_entry.file.clone()));
        previous_by_mod_id.insert(item.mod_id.clone(), previous);
    }

    let mut installed = Vec::<InstalledModResult>::new();
    let mut persisted = Vec::<InstalledModResult>::new();

    for item in &mods {
        let version_info = get_mod_version_info(&client, &api_base_url, &item.mod_id, &item.version).await?;
        let target = resolve_download_target(
            &api_base_url,
            &item.mod_id,
            &item.version,
            &version_info,
            game_platform.clone(),
        );

        let plugins_dir = PathBuf::from(profile_path).join("BepInEx").join("plugins");
        std::fs::create_dir_all(&plugins_dir)?;
        let destination = plugins_dir.join(&target.file_name);

        if let Err(error) = mod_download_service::download_mod(
            app.clone(),
            item.mod_id.clone(),
            target.url,
            destination.to_string_lossy().to_string(),
            target.checksum,
        )
        .await
        {
            rollback_install(
                &app,
                profile_id,
                profile_path,
                &installed,
                &persisted,
                &previous_by_mod_id,
            )?;
            return Err(error);
        }

        installed.push(InstalledModResult {
            mod_id: item.mod_id.clone(),
            version: item.version.clone(),
            file_name: target.file_name.clone(),
        });

        if let Err(error) = profile_service::add_mod_to_profile(
            &app,
            profile_id,
            &item.mod_id,
            &item.version,
            &target.file_name,
        ) {
            rollback_install(
                &app,
                profile_id,
                profile_path,
                &installed,
                &persisted,
                &previous_by_mod_id,
            )?;
            return Err(error);
        }

        persisted.push(InstalledModResult {
            mod_id: item.mod_id.clone(),
            version: item.version.clone(),
            file_name: target.file_name.clone(),
        });

        if let Some(Some((_version, Some(old_file)))) = previous_by_mod_id.get(&item.mod_id)
            && old_file != &target.file_name
        {
            let _ = profile_service::delete_mod_file(profile_path, old_file);
            }
    }

    Ok(installed)
}

fn rollback_install<R: Runtime>(
    app: &AppHandle<R>,
    profile_id: &str,
    profile_path: &str,
    installed: &[InstalledModResult],
    persisted: &[InstalledModResult],
    previous_by_mod_id: &HashMap<String, Option<(String, Option<String>)>>,
) -> AppResult<()> {
    for item in persisted.iter().rev() {
        if let Some(previous) = previous_by_mod_id.get(&item.mod_id) {
            match previous {
                Some((version, Some(file))) => {
                    let _ = profile_service::add_mod_to_profile(app, profile_id, &item.mod_id, version, file);
                }
                _ => {
                    let _ = profile_service::remove_mod_from_profile(app, profile_id, &item.mod_id);
                }
            }
        }
    }

    for item in installed.iter().rev() {
        let _ = profile_service::delete_mod_file(profile_path, &item.file_name);
    }
    Ok(())
}
