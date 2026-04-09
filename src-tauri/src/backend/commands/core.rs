use crate::backend::{
    commands::blocking::run_blocking,
    services::core_service::{self, AppSettings, AppSettingsPatch},
};
use tauri::{AppHandle, Runtime};

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoreUpdateSettingsArgs {
    pub updates: AppSettingsPatch,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoreAutoDetectBepInExArchitectureArgs {
    pub game_path: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoreGetBepInExCachePathArgs {
    pub architecture: String,
}

#[tauri::command]
pub async fn core_get_settings<R: Runtime>(app: AppHandle<R>) -> Result<AppSettings, String> {
    run_blocking(move || core_service::get_settings(&app).map_err(|e| e.to_string())).await
}

#[tauri::command]
pub async fn core_update_settings<R: Runtime>(
    app: AppHandle<R>,
    args: CoreUpdateSettingsArgs,
) -> Result<AppSettings, String> {
    run_blocking(move || {
        core_service::update_settings(&app, args.updates).map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
pub async fn core_get_bepinex_cache_path<R: Runtime>(app: AppHandle<R>) -> Result<String, String> {
    core_service::get_bepinex_cache_path(&app, "x86").map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn core_get_bepinex_cache_path_for_arch<R: Runtime>(
    app: AppHandle<R>,
    args: CoreGetBepInExCachePathArgs,
) -> Result<String, String> {
    core_service::get_bepinex_cache_path(&app, &args.architecture).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn core_get_app_data_dir<R: Runtime>(app: AppHandle<R>) -> Result<String, String> {
    core_service::get_app_data_dir(&app).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn core_auto_detect_bepinex_architecture<R: Runtime>(
    app: AppHandle<R>,
    args: CoreAutoDetectBepInExArchitectureArgs,
) -> Result<Option<String>, String> {
    run_blocking(move || {
        core_service::auto_detect_bepinex_architecture(&app, &args.game_path)
            .map_err(|e| e.to_string())
    })
    .await
}
