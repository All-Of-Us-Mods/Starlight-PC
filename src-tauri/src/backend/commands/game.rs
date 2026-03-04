use crate::backend::services::launch_service::{self, LaunchModdedArgs, LaunchVanillaArgs};
use crate::backend::services::xbox_service::{
    self, XboxCleanupArgs, XboxLaunchArgs, XboxPrepareLaunchArgs,
};
use tauri::{AppHandle, Runtime};

#[tauri::command]
pub async fn game_launch_modded<R: Runtime>(
    app: AppHandle<R>,
    args: LaunchModdedArgs,
) -> Result<(), String> {
    launch_service::launch_modded(app, args)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn game_launch_vanilla<R: Runtime>(
    app: AppHandle<R>,
    args: LaunchVanillaArgs,
) -> Result<(), String> {
    launch_service::launch_vanilla(app, args)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn game_xbox_get_app_id() -> Result<String, String> {
    xbox_service::get_xbox_app_id().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn game_xbox_prepare_launch(args: XboxPrepareLaunchArgs) -> Result<(), String> {
    xbox_service::prepare_xbox_launch(args).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn game_xbox_launch<R: Runtime>(
    app: AppHandle<R>,
    args: XboxLaunchArgs,
) -> Result<(), String> {
    xbox_service::launch_xbox(app, args).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn game_xbox_cleanup(args: XboxCleanupArgs) -> Result<(), String> {
    xbox_service::cleanup_xbox_files(args).map_err(|e| e.to_string())
}
