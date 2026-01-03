use crate::utils::game::extract_game_version;
use std::fs;
use std::path::Path;
use tauri::{AppHandle, Manager, Runtime};

#[tauri::command]
pub async fn save_game_copy<R: Runtime>(app: AppHandle<R>, path: String) -> Result<(), String> {
    let game_path = Path::new(&path);
    

    let version = extract_game_version(game_path)?;
    
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let versions_dir = app_dir.join("AmongUsVersions");
    let target_dir = versions_dir.join(&version);
    
    if target_dir.exists() {
        return Ok(());
    }
    
    copy_dir(game_path, &target_dir).map_err(|e| e.to_string())?;
    
    Ok(())
}

fn copy_dir(src: &Path, dst: &Path) -> std::io::Result<()> {
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }
    
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        
        if ty.is_dir() {
            copy_dir(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}
