use crate::backend::error::{AppError, AppResult};
use log::{debug, info, warn};
use std::path::{Path, PathBuf};

#[cfg(target_os = "windows")]
use winreg::{RegKey, enums::*};

const AMONG_US_EXE: &str = "Among Us.exe";
const AMONG_US_STEAM_APP_ID: &str = "945360";
const EPIC_FOLDER: &str = "Among Us_Data/StreamingAssets/aa/EGS";
const XBOX_FOLDER: &str = "Among Us_Data/StreamingAssets/aa/Win10";

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinuxRunnerDetection {
    pub runner_kind: String,
    pub runner_binary: Option<String>,
    pub wine_prefix: Option<String>,
    pub proton_compat_data_path: Option<String>,
    pub proton_steam_client_path: Option<String>,
    pub proton_use_steam_run: bool,
}

fn verify_among_us_directory(path: &Path) -> bool {
    path.is_dir() && path.join(AMONG_US_EXE).is_file()
}

#[cfg(target_os = "windows")]
fn is_windows_apps_path(path: &Path) -> bool {
    path.to_string_lossy()
        .to_lowercase()
        .contains("windowsapps")
}

fn is_epic_installation(path: &Path) -> bool {
    path.join(EPIC_FOLDER).is_dir()
}

fn is_xbox_installation(path: &Path) -> bool {
    path.join(XBOX_FOLDER).is_dir()
}

#[cfg(target_os = "windows")]
fn parse_registry_icon_value(raw_value: &str) -> Option<PathBuf> {
    let path = raw_value
        .split(',')
        .next()?
        .trim()
        .trim_matches(|c| c == '"' || c == '\'')
        .replace(';', "\\");

    if path.is_empty() {
        return None;
    }

    PathBuf::from(path).parent().map(|p| p.to_path_buf())
}

#[cfg(target_os = "windows")]
fn find_among_us_from_registry() -> Option<PathBuf> {
    let hkcr = RegKey::predef(HKEY_CLASSES_ROOT);

    for key_name in ["AmongUs", "amongus"] {
        let directory = hkcr
            .open_subkey(key_name)
            .ok()
            .and_then(|key| key.open_subkey("DefaultIcon").ok())
            .and_then(|icon_key| icon_key.get_value::<String, _>("").ok())
            .and_then(|raw_value| parse_registry_icon_value(&raw_value))
            .filter(|directory| verify_among_us_directory(directory));

        if let Some(dir) = directory {
            if is_windows_apps_path(&dir) {
                info!(
                    "Skipping WindowsApps path (Xbox installation): {}",
                    dir.display()
                );
                continue;
            }
            info!("Found Among Us via registry: {}", dir.display());
            return Some(dir);
        }
    }
    None
}

#[cfg(target_os = "linux")]
fn find_among_us_linux_paths() -> Vec<PathBuf> {
    fn parse_libraryfolders_paths(raw: &str) -> Vec<PathBuf> {
        let mut libraries = Vec::new();
        for line in raw.lines() {
            if !line.contains("\"path\"") {
                continue;
            }

            let mut parts = line.split('"');
            let _ = parts.next();
            let key = parts.next();
            let _ = parts.next();
            let value = parts.next();

            if key == Some("path") {
                let path = value.unwrap_or_default().replace("\\\\", "\\");
                if !path.is_empty() {
                    libraries.push(PathBuf::from(path));
                }
            }
        }
        libraries
    }

    let mut detected_paths = Vec::new();
    let mut steam_roots = Vec::new();
    if let Some(home) = home::home_dir() {
        steam_roots.push(home.join(".local/share/Steam"));
        steam_roots.push(home.join(".steam/steam"));
        steam_roots.push(home.join(".var/app/com.valvesoftware.Steam/data/Steam"));
    }

    let mut library_roots = Vec::new();
    for steam_root in &steam_roots {
        library_roots.push(steam_root.clone());
        let library_folders_vdf = steam_root.join("steamapps").join("libraryfolders.vdf");
        if let Ok(raw) = std::fs::read_to_string(library_folders_vdf) {
            library_roots.extend(parse_libraryfolders_paths(&raw));
        }
    }

    library_roots.sort();
    library_roots.dedup();

    for library_root in library_roots {
        let full_path = library_root.join("steamapps").join("common").join("Among Us");
        if verify_among_us_directory(&full_path) {
            info!("Found Among Us at: {}", full_path.display());
            detected_paths.push(full_path);
        }
    }

    detected_paths
}

#[cfg(target_os = "linux")]
fn linux_steam_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(home) = home::home_dir() {
        roots.push(home.join(".local/share/Steam"));
        roots.push(home.join(".steam/steam"));
        roots.push(home.join(".var/app/com.valvesoftware.Steam/data/Steam"));
    }
    roots
}

#[cfg(target_os = "linux")]
fn proton_binary_candidates(steam_root: &Path) -> Vec<PathBuf> {
    let proton_common = steam_root.join("steamapps").join("common");
    let mut candidates = Vec::new();

    let preferred = proton_common.join("Proton - Experimental").join("proton");
    if preferred.is_file() {
        candidates.push(preferred);
    }

    if let Ok(entries) = std::fs::read_dir(&proton_common) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let Some(name) = path.file_name().and_then(|v| v.to_str()) else {
                continue;
            };
            if name.starts_with("Proton") {
                let binary = path.join("proton");
                if binary.is_file() {
                    candidates.push(binary);
                }
            }
        }
    }

    candidates
}

#[cfg(target_os = "linux")]
fn pick_proton_binary(steam_root: &Path) -> Option<PathBuf> {
    proton_binary_candidates(steam_root)
        .into_iter()
        .max_by_key(|path| {
            std::fs::metadata(path)
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0)
        })
}

#[cfg(target_os = "linux")]
fn steamapps_root_from_among_us_path(path: &Path) -> Option<PathBuf> {
    let mut current = Some(path);
    while let Some(dir) = current {
        if dir.file_name().and_then(|v| v.to_str()) == Some("steamapps") {
            return Some(dir.to_path_buf());
        }
        current = dir.parent();
    }
    None
}

#[cfg(target_os = "linux")]
fn detect_linux_runner_from_among_us(path: &Path) -> LinuxRunnerDetection {
    let steamapps_root = steamapps_root_from_among_us_path(path);
    let compat_data = steamapps_root
        .as_ref()
        .map(|root| root.join("compatdata").join(AMONG_US_STEAM_APP_ID));

    let steam_root = steamapps_root
        .as_ref()
        .and_then(|root| root.parent().map(|p| p.to_path_buf()))
        .or_else(|| linux_steam_roots().into_iter().find(|p| p.exists()));

    let proton_binary = steam_root
        .as_ref()
        .and_then(|root| pick_proton_binary(root))
        .map(|p| p.to_string_lossy().to_string());

    LinuxRunnerDetection {
        runner_kind: "proton".to_string(),
        runner_binary: proton_binary,
        wine_prefix: None,
        proton_compat_data_path: compat_data.map(|p| p.to_string_lossy().to_string()),
        proton_steam_client_path: steam_root.map(|p| p.to_string_lossy().to_string()),
        proton_use_steam_run: true,
    }
}

#[cfg(target_os = "linux")]
fn detect_linux_runner_fallback() -> LinuxRunnerDetection {
    let steam_root = linux_steam_roots().into_iter().find(|p| p.exists());
    let proton_binary = steam_root
        .as_ref()
        .and_then(|root| pick_proton_binary(root))
        .map(|p| p.to_string_lossy().to_string());

    LinuxRunnerDetection {
        runner_kind: "proton".to_string(),
        runner_binary: proton_binary,
        wine_prefix: None,
        proton_compat_data_path: steam_root
            .as_ref()
            .map(|root| root.join("steamapps").join("compatdata").join(AMONG_US_STEAM_APP_ID))
            .map(|p| p.to_string_lossy().to_string()),
        proton_steam_client_path: steam_root.map(|p| p.to_string_lossy().to_string()),
        proton_use_steam_run: true,
    }
}

pub fn detect_among_us_installation() -> AppResult<Option<String>> {
    let paths = get_among_us_paths();
    Ok(paths.first().map(|p| p.to_string_lossy().to_string()))
}

pub fn get_among_us_paths() -> Vec<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        if let Some(path) = find_among_us_from_registry() {
            return vec![path];
        }
    }

    #[cfg(target_os = "linux")]
    {
        let paths = find_among_us_linux_paths();
        if !paths.is_empty() {
            return paths;
        }
    }

    info!("Among Us installation not detected");
    Vec::new()
}

pub fn detect_game_store(path: &str) -> AppResult<String> {
    let path = PathBuf::from(path);

    if !verify_among_us_directory(&path) {
        warn!("Invalid Among Us installation directory: {:?}", path);
        return Err(AppError::platform(
            "Invalid Among Us installation directory",
        ));
    }

    let platform = if is_epic_installation(&path) {
        "epic"
    } else if is_xbox_installation(&path) {
        "xbox"
    } else {
        "steam"
    };

    debug!("Detected platform '{}' for path: {:?}", platform, path);
    Ok(platform.to_string())
}

#[cfg(target_os = "linux")]
pub fn detect_linux_runner(path: Option<String>) -> AppResult<LinuxRunnerDetection> {
    if let Some(path) = path {
        let game_path = PathBuf::from(path);
        if verify_among_us_directory(&game_path) {
            return Ok(detect_linux_runner_from_among_us(&game_path));
        }
    }

    if let Some(among_us_path) = get_among_us_paths().first() {
        return Ok(detect_linux_runner_from_among_us(among_us_path));
    }

    Ok(detect_linux_runner_fallback())
}

#[cfg(test)]
mod tests {
    use super::detect_game_store;

    #[test]
    fn detect_store_rejects_invalid_path() {
        let result = detect_game_store("/definitely/not/a/real/amoungus/path");
        assert!(result.is_err());
    }
}
