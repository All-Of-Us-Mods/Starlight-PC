use crate::utils::finder::get_among_us_paths;

#[tauri::command]
pub fn detect_among_us() -> Result<String, String> {
    let paths = get_among_us_paths();
    
    paths
        .first()
        .map(|path| path.to_string_lossy().to_string())
        .ok_or_else(|| "Among Us installation not found".to_string())
}
