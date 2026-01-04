use crate::utils::finder::{detect_platform, get_among_us_paths};
use log::{debug, info, warn};

#[tauri::command]
pub fn detect_among_us() -> Result<String, String> {
    info!("Detecting Among Us installation");
    let paths = get_among_us_paths();

    paths
        .first()
        .map(|path| {
            let path_str = path.to_string_lossy().to_string();
            info!("Among Us detected at: {}", path_str);
            path_str
        })
        .ok_or_else(|| {
            warn!("Among Us installation not found");
            "Among Us installation not found".to_string()
        })
}

#[tauri::command]
pub fn get_game_platform(path: String) -> Result<String, String> {
    debug!("Detecting game platform for: {}", path);
    let platform = detect_platform(&path)?;
    info!("Game platform detected: {} for path: {}", platform, path);
    Ok(platform)
}
