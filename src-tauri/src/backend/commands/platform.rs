use crate::backend::services::finder_service;
use log::info;

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformDetectGameStoreArgs {
    pub path: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformDetectLinuxRunnerArgs {
    pub path: Option<String>,
}

#[tauri::command]
pub fn platform_detect_among_us() -> Result<Option<String>, String> {
    finder_service::detect_among_us_installation().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn platform_detect_game_store(args: PlatformDetectGameStoreArgs) -> Result<String, String> {
    let platform = finder_service::detect_game_store(&args.path).map_err(|e| e.to_string())?;
    info!(
        "Game platform detected: {} for path: {}",
        platform, args.path
    );
    Ok(platform)
}

#[tauri::command]
pub fn platform_detect_linux_runner(
    args: PlatformDetectLinuxRunnerArgs,
) -> Result<finder_service::LinuxRunnerDetection, String> {
    #[cfg(target_os = "linux")]
    {
        finder_service::detect_linux_runner(args.path).map_err(|e| e.to_string())
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = args;
        Err("Linux runner detection is only available on Linux".to_string())
    }
}
