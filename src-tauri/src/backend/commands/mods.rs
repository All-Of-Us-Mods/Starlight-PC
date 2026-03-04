use crate::backend::services::bepinex_service;
use crate::backend::services::mod_download_service;
use tauri::{AppHandle, Runtime};

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModdingBepInExInstallArgs {
    pub url: String,
    pub destination: String,
    pub cache_path: Option<String>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModdingBepInExCacheDownloadArgs {
    pub url: String,
    pub cache_path: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModdingBepInExCacheClearArgs {
    pub cache_path: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModdingBepInExCacheExistsArgs {
    pub cache_path: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModdingModDownloadArgs {
    pub mod_id: String,
    pub url: String,
    pub destination: String,
    pub expected_checksum: String,
}

#[tauri::command]
pub async fn modding_bepinex_install<R: Runtime>(
    app: AppHandle<R>,
    args: ModdingBepInExInstallArgs,
) -> Result<(), String> {
    bepinex_service::install_bepinex(app, args.url, args.destination, args.cache_path)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn modding_bepinex_cache_download<R: Runtime>(
    app: AppHandle<R>,
    args: ModdingBepInExCacheDownloadArgs,
) -> Result<(), String> {
    bepinex_service::download_bepinex_to_cache(app, args.url, args.cache_path)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn modding_bepinex_cache_clear(args: ModdingBepInExCacheClearArgs) -> Result<(), String> {
    bepinex_service::clear_cache(args.cache_path).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn modding_bepinex_cache_exists(
    args: ModdingBepInExCacheExistsArgs,
) -> Result<bool, String> {
    Ok(bepinex_service::cache_exists(args.cache_path))
}

#[tauri::command]
pub async fn modding_mod_download<R: Runtime>(
    app: AppHandle<R>,
    args: ModdingModDownloadArgs,
) -> Result<(), String> {
    mod_download_service::download_mod(
        app,
        args.mod_id,
        args.url,
        args.destination,
        args.expected_checksum,
    )
    .await
    .map_err(|e| e.to_string())
}
