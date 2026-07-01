//! Self-update check against GitHub Releases, Zed-style: check the latest
//! release tag against the running version, and if newer, download the
//! Windows exe and swap it in for the next launch.
//!
//! Windows-only for now — swapping the running executable relies on the
//! quirk that Windows allows renaming (but not overwriting) an in-use file.

use crate::backend::error::{AppError, AppResult};
use log::info;
use serde::Deserialize;
use std::time::Duration;

const RELEASES_API_URL: &str =
    "https://api.github.com/repos/All-Of-Us-Mods/Starlight-PC/releases/latest";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Debug, Clone)]
pub struct UpdateInfo {
    pub version: String,
    pub download_url: String,
}

#[derive(Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Deserialize)]
struct GithubRelease {
    tag_name: String,
    assets: Vec<GithubAsset>,
}

/// Check the latest GitHub release against the running version. Returns
/// `Ok(None)` if we're already up to date or the release has no Windows
/// asset to offer.
pub fn check_for_update() -> AppResult<Option<UpdateInfo>> {
    info!("checking for updates against {RELEASES_API_URL}");

    let client = reqwest::blocking::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()?;

    let release: GithubRelease = client
        .get(RELEASES_API_URL)
        .header("User-Agent", "Starlight-Updater")
        .send()?
        .error_for_status()?
        .json()?;

    let tag = release.tag_name.trim_start_matches('v');
    let latest = semver::Version::parse(tag)
        .map_err(|e| AppError::parse(format!("Invalid release tag '{tag}': {e}")))?;
    let current = semver::Version::parse(env!("CARGO_PKG_VERSION"))
        .expect("CARGO_PKG_VERSION is valid semver");

    if latest <= current {
        info!("up to date (running {current}, latest release is {latest})");
        return Ok(None);
    }

    let Some(asset) = release
        .assets
        .iter()
        .find(|a| a.name.eq_ignore_ascii_case("Starlight-windows-x86_64.exe"))
    else {
        info!("release {latest} has no Windows asset, skipping");
        return Ok(None);
    };

    info!("update available: {current} -> {latest}");

    Ok(Some(UpdateInfo {
        version: latest.to_string(),
        download_url: asset.browser_download_url.clone(),
    }))
}

/// Download the new exe and swap it in place of the running one, then
/// launch it. Windows allows renaming an open/running executable (just not
/// overwriting its contents in place), so: rename the running exe aside,
/// move the downloaded exe into its place, then spawn it. The caller is
/// responsible for quitting the current process afterwards.
#[cfg(windows)]
pub fn apply_update_and_relaunch(info: &UpdateInfo) -> AppResult<()> {
    use crate::backend::services::http_download;
    use std::fs;
    use std::process::Command;

    let current_exe = std::env::current_exe()?;
    let download_path = current_exe.with_extension("download.exe");
    let old_path = current_exe.with_extension("old.exe");

    info!(
        "downloading update {} from {}",
        info.version, info.download_url
    );
    http_download::download_file(&info.download_url, &download_path, |_, _| {})?;

    let _ = fs::remove_file(&old_path);
    fs::rename(&current_exe, &old_path)?;
    if let Err(e) = fs::rename(&download_path, &current_exe) {
        // Best-effort rollback so the user isn't left without an exe.
        let _ = fs::rename(&old_path, &current_exe);
        return Err(e.into());
    }

    info!("update {} installed, relaunching", info.version);
    Command::new(&current_exe).spawn()?;

    Ok(())
}

/// Delete a leftover `.old.exe` from a previous update. The rename in
/// [`apply_update_and_relaunch`] leaves the old exe locked until the process
/// that was running it exits, so cleanup happens on the next launch instead.
#[cfg(windows)]
pub fn cleanup_leftover_old_exe() {
    if let Ok(current_exe) = std::env::current_exe() {
        let old_path = current_exe.with_extension("old.exe");
        if std::fs::remove_file(&old_path).is_ok() {
            info!("removed leftover {}", old_path.display());
        }
    }
}
