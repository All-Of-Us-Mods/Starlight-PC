use std::fs;
use std::path::Path;

pub fn extract_game_version(game_path: &Path) -> Result<String, String> {
    let file_path = game_path
        .join("Among Us_Data")
        .join("globalgamemanagers");

    if !file_path.exists() {
        return Err(format!(
            "globalgamemanagers file not found at: {}",
            file_path.display()
        ));
    }

    let bytes = fs::read(&file_path).map_err(|e| format!("Failed to read file: {}", e))?;

    let pattern = b"public.app-category.games";
    let index = find_pattern(&bytes, pattern)
        .ok_or("Version pattern not found in globalgamemanagers")?
        + pattern.len();

    let remaining = &bytes[index..];
    let version_pattern = b"20";
    let version_index = find_pattern(remaining, version_pattern)
        .ok_or("Version number not found in globalgamemanagers")?;

    let version_start = index + version_index;

    let version_bytes: Vec<u8> = bytes[version_start..]
        .iter()
        .take_while(|&&b| b != 0)
        .copied()
        .collect();

    String::from_utf8(version_bytes)
        .map_err(|e| format!("Failed to parse version string: {}", e))
}

fn find_pattern(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}
