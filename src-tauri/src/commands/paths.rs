use crate::finder;
use log::info;

#[tauri::command]
pub async fn get_among_us_paths() -> Result<Vec<String>, String> {
    let paths = finder::get_among_us_paths();
    let resolved: Vec<String> = paths
        .into_iter()
        .filter_map(|path| path.to_str().map(|s| s.to_string()))
        .collect();

    if resolved.is_empty() {
        Err("Among Us path not found".to_string())
    } else {
        info!("Resolved Among Us paths: {:?}", resolved);
        Ok(resolved)
    }
}
