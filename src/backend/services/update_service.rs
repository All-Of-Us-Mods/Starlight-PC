//! Self-update check against GitHub Releases, Zed-style: check the newest
//! release on the user's channel against the running version, and if newer,
//! download the Windows exe and swap it in for the next launch.
//!
//! Windows-only for now — swapping the running executable relies on the
//! quirk that Windows allows renaming (but not overwriting) an in-use file.
//!
//! Two channels (see [`ReleaseChannel`]): stable follows the tagged releases,
//! nightly also picks up the pre-releases the nightly workflow cuts from
//! `main`. Nightly tags are pre-releases of the *next* patch version
//! (`2.1.1-nightly.20260909.42` when 2.1.0 is out), so a nightly always sorts
//! above the stable it was cut from and below the stable that supersedes it —
//! which is what keeps a nightly user moving forward and lets them land back
//! on stable when that version ships.

use crate::backend::error::{AppError, AppResult};
use crate::backend::services::core_service::ReleaseChannel;
use log::info;
use serde::Deserialize;
use std::time::Duration;

const REPO_API_URL: &str = "https://api.github.com/repos/All-Of-Us-Mods/Starlight-PC";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
const RELEASE_DOWNLOAD_PREFIX: &str =
    "https://github.com/All-Of-Us-Mods/Starlight-PC/releases/download/";
/// The one asset the updater can install. Keep in sync with the release
/// workflows' artifact names.
const WINDOWS_ASSET_NAME: &str = "Starlight-windows-x86_64.exe";

#[derive(Debug, Clone)]
pub struct UpdateInfo {
    pub version: String,
    pub download_url: String,
    pub expected_sha256: Option<String>,
}

#[derive(Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
    digest: Option<String>,
}

#[derive(Deserialize)]
struct GithubRelease {
    tag_name: String,
    #[serde(default)]
    draft: bool,
    assets: Vec<GithubAsset>,
}

impl GithubRelease {
    fn windows_asset(&self) -> Option<&GithubAsset> {
        self.assets
            .iter()
            .find(|a| a.name.eq_ignore_ascii_case(WINDOWS_ASSET_NAME))
    }
}

/// Parse a GitHub asset digest of the form "sha256:<64 hex chars>" into
/// the lowercase hex hash. Returns None for any other shape.
fn parse_sha256_digest(digest: &str) -> Option<String> {
    if digest.len() < 7 || !digest[..7].eq_ignore_ascii_case("sha256:") {
        return None;
    }
    let hex = &digest[7..];
    if hex.len() == 64 && hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        Some(hex.to_lowercase())
    } else {
        None
    }
}

/// The release to offer: the highest version that actually ships the Windows
/// asset. Drafts and tags that aren't semver are skipped, and so is a release
/// whose asset upload didn't land — offering that would hand the user a
/// notification that can't be installed.
fn best_release(releases: &[GithubRelease]) -> Option<(semver::Version, &GithubRelease)> {
    releases
        .iter()
        .filter(|r| !r.draft && r.windows_asset().is_some())
        .filter_map(|r| {
            let version = semver::Version::parse(r.tag_name.trim_start_matches('v')).ok()?;
            Some((version, r))
        })
        .max_by(|(a, _), (b, _)| a.cmp(b))
}

/// Whether `candidate` is worth offering to someone running `current`.
///
/// Normally that means "newer". The exception is a user who switched from
/// nightly back to stable: the newest stable is *older* than the nightly they
/// are running, and without this they'd be stuck on the nightly channel until
/// the next stable release caught up.
fn is_upgrade(
    current: &semver::Version,
    candidate: &semver::Version,
    channel: ReleaseChannel,
) -> bool {
    candidate > current
        || (channel == ReleaseChannel::Stable && !current.pre.is_empty() && candidate != current)
}

fn fetch<T: serde::de::DeserializeOwned>(url: &str) -> AppResult<T> {
    let client =
        crate::backend::services::http_download::http_client(REQUEST_TIMEOUT, REQUEST_TIMEOUT)?;

    Ok(client
        .get(url)
        .header("User-Agent", "Starlight-Updater")
        .send()?
        .error_for_status()?
        .json()?)
}

/// Check the user's channel for a newer build than the running version.
/// Returns `Ok(None)` if we're already up to date or nothing on the channel
/// has a Windows asset to offer.
pub fn check_for_update(channel: ReleaseChannel) -> AppResult<Option<UpdateInfo>> {
    let current = semver::Version::parse(env!("CARGO_PKG_VERSION"))
        .expect("CARGO_PKG_VERSION is valid semver");

    // `/releases/latest` is GitHub's "newest non-pre-release", which is
    // exactly the stable channel. Nightlies are pre-releases, so the nightly
    // channel reads the release list instead and picks the highest version
    // itself — including stable ones, so a nightly user still lands on a
    // release when it supersedes their build.
    let releases: Vec<GithubRelease> = match channel {
        ReleaseChannel::Stable => {
            let url = format!("{REPO_API_URL}/releases/latest");
            info!("checking for updates against {url}");
            vec![fetch(&url)?]
        }
        ReleaseChannel::Nightly => {
            // 30 covers a month of nightlies, so the newest is always in here.
            let url = format!("{REPO_API_URL}/releases?per_page=30");
            info!("checking for updates against {url}");
            fetch(&url)?
        }
    };

    let Some((latest, release)) = best_release(&releases) else {
        info!("no installable release found on the {channel:?} channel");
        return Ok(None);
    };

    if !is_upgrade(&current, &latest, channel) {
        info!("up to date (running {current}, newest on {channel:?} is {latest})");
        return Ok(None);
    }

    let asset = release
        .windows_asset()
        .expect("best_release only returns releases with a Windows asset");

    if !asset
        .browser_download_url
        .starts_with(RELEASE_DOWNLOAD_PREFIX)
    {
        return Err(AppError::validation(format!(
            "Unexpected update download URL: {}",
            asset.browser_download_url
        )));
    }

    info!("update available on {channel:?}: {current} -> {latest}");

    Ok(Some(UpdateInfo {
        version: latest.to_string(),
        download_url: asset.browser_download_url.clone(),
        expected_sha256: asset.digest.as_deref().and_then(parse_sha256_digest),
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
    http_download::download_file(&info.download_url, &download_path, None, None, |_, _| {})?;

    let Some(expected) = info.expected_sha256.as_deref() else {
        let _ = fs::remove_file(&download_path);
        return Err(AppError::validation(
            "Release asset has no sha256 digest; refusing to install update",
        ));
    };

    let computed = hash_file_sha256(&download_path)?;
    if computed != expected {
        let _ = fs::remove_file(&download_path);
        return Err(AppError::validation(format!(
            "Update checksum mismatch: expected {expected}, got {computed}"
        )));
    }

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

#[cfg(windows)]
fn hash_file_sha256(path: &std::path::Path) -> AppResult<String> {
    use sha2::{Digest, Sha256};
    use std::io::Read;

    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut chunk = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut chunk)?;
        if n == 0 {
            break;
        }
        hasher.update(&chunk[..n]);
    }
    Ok(crate::backend::services::hex_digest(hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Releases as GitHub returns them, so the tests cover the JSON shape
    /// the updater depends on as well as the picking logic.
    fn releases(json: &str) -> Vec<GithubRelease> {
        serde_json::from_str(json).expect("valid release list")
    }

    fn asset(name: &str) -> String {
        format!(
            r#"{{"name": "{name}", "browser_download_url": "{RELEASE_DOWNLOAD_PREFIX}v1.0.0/{name}", "digest": null}}"#
        )
    }

    fn release(tag: &str, draft: bool, assets: &[&str]) -> String {
        let assets: Vec<String> = assets.iter().map(|n| asset(n)).collect();
        format!(
            r#"{{"tag_name": "{tag}", "draft": {draft}, "assets": [{}]}}"#,
            assets.join(",")
        )
    }

    fn version(v: &str) -> semver::Version {
        semver::Version::parse(v).expect("valid version")
    }

    #[test]
    fn best_release_picks_the_highest_version() {
        let list = releases(&format!(
            "[{},{},{}]",
            release("v2.1.0", false, &[WINDOWS_ASSET_NAME]),
            release("v2.2.1-nightly.20260909.42", false, &[WINDOWS_ASSET_NAME]),
            release("v2.2.1-nightly.20260908.41", false, &[WINDOWS_ASSET_NAME]),
        ));
        let (picked, _) = best_release(&list).expect("a release");
        assert_eq!(picked, version("2.2.1-nightly.20260909.42"));
    }

    #[test]
    fn best_release_skips_drafts_and_releases_without_the_windows_asset() {
        let list = releases(&format!(
            "[{},{},{}]",
            release("v3.0.0", true, &[WINDOWS_ASSET_NAME]),
            release("v2.9.0", false, &["Starlight-linux-x86_64"]),
            release("v2.1.0", false, &[WINDOWS_ASSET_NAME]),
        ));
        let (picked, _) = best_release(&list).expect("a release");
        assert_eq!(picked, version("2.1.0"));
    }

    #[test]
    fn best_release_skips_tags_that_are_not_semver() {
        let list = releases(&format!(
            "[{},{}]",
            release("nightly", false, &[WINDOWS_ASSET_NAME]),
            release("v2.1.0", false, &[WINDOWS_ASSET_NAME]),
        ));
        let (picked, _) = best_release(&list).expect("a release");
        assert_eq!(picked, version("2.1.0"));
    }

    #[test]
    fn best_release_returns_none_when_nothing_is_installable() {
        let list = releases(&format!("[{}]", release("v2.1.0", true, &[])));
        assert!(best_release(&list).is_none());
    }

    #[test]
    fn nightly_sorts_between_the_stable_it_follows_and_the_next_one() {
        assert!(version("2.1.0") < version("2.1.1-nightly.20260909.42"));
        assert!(version("2.1.1-nightly.20260909.42") < version("2.1.1"));
    }

    #[test]
    fn is_upgrade_accepts_newer_and_rejects_same_or_older() {
        for channel in [ReleaseChannel::Stable, ReleaseChannel::Nightly] {
            assert!(is_upgrade(&version("2.1.0"), &version("2.1.1"), channel));
            assert!(!is_upgrade(&version("2.1.0"), &version("2.1.0"), channel));
            assert!(!is_upgrade(&version("2.1.1"), &version("2.1.0"), channel));
        }
    }

    #[test]
    fn is_upgrade_moves_a_nightly_back_onto_the_stable_channel() {
        let nightly = version("2.1.1-nightly.20260909.42");
        let stable = version("2.1.0");
        assert!(is_upgrade(&nightly, &stable, ReleaseChannel::Stable));
        // On the nightly channel the same pair is a downgrade, so it isn't
        // offered — the next nightly will be.
        assert!(!is_upgrade(&nightly, &stable, ReleaseChannel::Nightly));
    }

    #[test]
    fn parse_sha256_digest_accepts_valid_digest() {
        let hex = "a".repeat(64);
        assert_eq!(
            parse_sha256_digest(&format!("sha256:{hex}")),
            Some(hex.clone())
        );
    }

    #[test]
    fn parse_sha256_digest_lowercases_uppercase_hex() {
        let upper = "A".repeat(64);
        let lower = "a".repeat(64);
        assert_eq!(parse_sha256_digest(&format!("sha256:{upper}")), Some(lower));
    }

    #[test]
    fn parse_sha256_digest_rejects_wrong_algorithm_prefix() {
        assert_eq!(
            parse_sha256_digest(&format!("sha512:{}", "a".repeat(64))),
            None
        );
    }

    #[test]
    fn parse_sha256_digest_rejects_wrong_length() {
        assert_eq!(
            parse_sha256_digest(&format!("sha256:{}", "a".repeat(63))),
            None
        );
    }

    #[test]
    fn parse_sha256_digest_rejects_non_hex_chars() {
        assert_eq!(
            parse_sha256_digest(&format!("sha256:{}", "z".repeat(64))),
            None
        );
    }

    #[test]
    fn parse_sha256_digest_rejects_empty_string() {
        assert_eq!(parse_sha256_digest(""), None);
    }
}
