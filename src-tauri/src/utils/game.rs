use log::{debug, error, info, warn};
use std::fs;
use std::path::Path;

pub fn extract_game_version(game_path: &Path) -> Result<String, String> {
    debug!("Extracting game version from: {:?}", game_path);
    let file_path = game_path.join("Among Us_Data").join("globalgamemanagers");

    if !file_path.exists() {
        let err = format!(
            "globalgamemanagers file not found at: {}",
            file_path.display()
        );
        error!("{}", err);
        return Err(err);
    }

    let bytes = fs::read(&file_path).map_err(|e| {
        let err = format!("Failed to read file: {}", e);
        error!("{}", err);
        err
    })?;

    let pattern = b"public.app-category.games";
    let index = find_pattern(&bytes, pattern).ok_or_else(|| {
        let err = "Version pattern not found in globalgamemanagers";
        warn!("{}", err);
        err.to_string()
    })? + pattern.len();

    let remaining = &bytes[index..];
    let version_pattern = b"20";
    let version_index = find_pattern(remaining, version_pattern).ok_or_else(|| {
        let err = "Version number not found in globalgamemanagers";
        warn!("{}", err);
        err.to_string()
    })?;

    let version_start = index + version_index;

    let version_bytes: Vec<u8> = bytes[version_start..]
        .iter()
        .take_while(|&&b| b != 0)
        .copied()
        .collect();

    let version = String::from_utf8(version_bytes).map_err(|e| {
        let err = format!("Failed to parse version string: {}", e);
        error!("{}", err);
        err
    })?;

    info!("Extracted game version: {}", version);
    Ok(version)
}

fn find_pattern(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}
