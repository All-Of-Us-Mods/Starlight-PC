use crate::backend::services::profile_service::{
    self, ProfileEntry, ProfileIconSelection, UnifiedMod,
};
use std::path::PathBuf;
use tauri::{AppHandle, Runtime};

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfilesGetByIdArgs {
    pub id: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfilesCreateArgs {
    pub name: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfilesDeleteArgs {
    pub profile_id: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfilesRenameArgs {
    pub profile_id: String,
    pub new_name: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfilesUpdateIconArgs {
    pub profile_id: String,
    pub selection: ProfileIconSelection,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfilesInstallBepInExArgs {
    pub profile_id: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfilesUpdateLastLaunchedArgs {
    pub profile_id: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfilesAddModArgs {
    pub profile_id: String,
    pub mod_id: String,
    pub version: String,
    pub file: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfilesAddPlayTimeArgs {
    pub profile_id: String,
    pub duration_ms: i64,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfilesRemoveModArgs {
    pub profile_id: String,
    pub mod_id: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfilesModFilesArgs {
    pub profile_path: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfilesDeleteModFileArgs {
    pub profile_path: String,
    pub file_name: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfilesGetLogArgs {
    pub profile_path: String,
    pub file_name: Option<String>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfilesReadBinaryFileArgs {
    pub path: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfilesDeleteUnifiedModArgs {
    pub profile_id: String,
    pub mod_entry: UnifiedMod,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfilesExportZipArgs {
    pub profile_id: String,
    pub destination: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfilesImportZipArgs {
    pub zip_path: String,
}

fn ensure_profile_path_in_profiles_dir<R: Runtime>(
    app: &AppHandle<R>,
    profile_path: &str,
) -> Result<(), String> {
    let allowed_root =
        PathBuf::from(profile_service::get_profiles_dir(app).map_err(|e| e.to_string())?)
            .canonicalize()
            .map_err(|_| "Invalid profiles directory".to_string())?;
    let canonical = PathBuf::from(profile_path)
        .canonicalize()
        .map_err(|_| "Invalid profile path".to_string())?;
    if !canonical.starts_with(&allowed_root) {
        return Err("Path is outside the allowed directory".to_string());
    }
    Ok(())
}

async fn run_blocking<T, F>(work: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|e| format!("Task failed: {e}"))?
}

#[tauri::command]
pub async fn profiles_get_dir<R: Runtime>(app: AppHandle<R>) -> Result<String, String> {
    run_blocking(move || profile_service::get_profiles_dir(&app).map_err(|e| e.to_string())).await
}

#[tauri::command]
pub async fn profiles_list<R: Runtime>(app: AppHandle<R>) -> Result<Vec<ProfileEntry>, String> {
    run_blocking(move || profile_service::get_profiles(&app).map_err(|e| e.to_string())).await
}

#[tauri::command]
pub async fn profiles_get_by_id<R: Runtime>(
    app: AppHandle<R>,
    args: ProfilesGetByIdArgs,
) -> Result<Option<ProfileEntry>, String> {
    run_blocking(move || {
        profile_service::get_profile_by_id(&app, &args.id).map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
pub async fn profiles_create<R: Runtime>(
    app: AppHandle<R>,
    args: ProfilesCreateArgs,
) -> Result<ProfileEntry, String> {
    run_blocking(move || {
        profile_service::create_profile(&app, &args.name).map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
pub async fn profiles_install_bepinex<R: Runtime>(
    app: AppHandle<R>,
    args: ProfilesInstallBepInExArgs,
) -> Result<(), String> {
    profile_service::install_bepinex_for_profile(app, &args.profile_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn profiles_delete<R: Runtime>(
    app: AppHandle<R>,
    args: ProfilesDeleteArgs,
) -> Result<(), String> {
    run_blocking(move || {
        profile_service::delete_profile(&app, &args.profile_id).map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
pub async fn profiles_rename<R: Runtime>(
    app: AppHandle<R>,
    args: ProfilesRenameArgs,
) -> Result<(), String> {
    run_blocking(move || {
        profile_service::rename_profile(&app, &args.profile_id, &args.new_name)
            .map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
pub async fn profiles_update_icon<R: Runtime>(
    app: AppHandle<R>,
    args: ProfilesUpdateIconArgs,
) -> Result<(), String> {
    run_blocking(move || {
        profile_service::update_profile_icon(&app, &args.profile_id, args.selection)
            .map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
pub async fn profiles_update_last_launched<R: Runtime>(
    app: AppHandle<R>,
    args: ProfilesUpdateLastLaunchedArgs,
) -> Result<(), String> {
    run_blocking(move || {
        profile_service::update_last_launched(&app, &args.profile_id).map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
pub async fn profiles_add_mod<R: Runtime>(
    app: AppHandle<R>,
    args: ProfilesAddModArgs,
) -> Result<(), String> {
    run_blocking(move || {
        profile_service::add_mod_to_profile(
            &app,
            &args.profile_id,
            &args.mod_id,
            &args.version,
            &args.file,
        )
        .map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
pub async fn profiles_add_play_time<R: Runtime>(
    app: AppHandle<R>,
    args: ProfilesAddPlayTimeArgs,
) -> Result<(), String> {
    run_blocking(move || {
        profile_service::add_play_time(&app, &args.profile_id, args.duration_ms)
            .map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
pub async fn profiles_remove_mod<R: Runtime>(
    app: AppHandle<R>,
    args: ProfilesRemoveModArgs,
) -> Result<(), String> {
    run_blocking(move || {
        profile_service::remove_mod_from_profile(&app, &args.profile_id, &args.mod_id)
            .map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
pub async fn profiles_get_mod_files<R: Runtime>(
    app: AppHandle<R>,
    args: ProfilesModFilesArgs,
) -> Result<Vec<String>, String> {
    run_blocking(move || {
        ensure_profile_path_in_profiles_dir(&app, &args.profile_path)?;
        Ok(profile_service::get_mod_files(&args.profile_path))
    })
    .await
}

#[tauri::command]
pub async fn profiles_delete_mod_file<R: Runtime>(
    app: AppHandle<R>,
    args: ProfilesDeleteModFileArgs,
) -> Result<(), String> {
    run_blocking(move || {
        ensure_profile_path_in_profiles_dir(&app, &args.profile_path)?;
        profile_service::delete_mod_file(&args.profile_path, &args.file_name)
            .map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
pub async fn profiles_get_log<R: Runtime>(
    app: AppHandle<R>,
    args: ProfilesGetLogArgs,
) -> Result<String, String> {
    run_blocking(move || {
        ensure_profile_path_in_profiles_dir(&app, &args.profile_path)?;
        Ok(profile_service::get_profile_log(
            &args.profile_path,
            args.file_name.as_deref().unwrap_or("LogOutput.log"),
        ))
    })
    .await
}

#[tauri::command]
pub async fn profiles_read_binary_file<R: Runtime>(
    app: AppHandle<R>,
    args: ProfilesReadBinaryFileArgs,
) -> Result<Vec<u8>, String> {
    run_blocking(move || {
        profile_service::read_binary_file(&app, &args.path).map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
pub async fn profiles_delete_unified_mod<R: Runtime>(
    app: AppHandle<R>,
    args: ProfilesDeleteUnifiedModArgs,
) -> Result<(), String> {
    run_blocking(move || {
        profile_service::delete_unified_mod(&app, &args.profile_id, args.mod_entry)
            .map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
pub async fn profiles_export_zip<R: Runtime>(
    app: AppHandle<R>,
    args: ProfilesExportZipArgs,
) -> Result<(), String> {
    let result = tauri::async_runtime::spawn_blocking(move || {
        profile_service::export_profile_zip(&app, &args.profile_id, &args.destination)
    })
    .await
    .map_err(|e| format!("Task failed: {e}"))?;

    result.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn profiles_import_zip<R: Runtime>(
    app: AppHandle<R>,
    args: ProfilesImportZipArgs,
) -> Result<ProfileEntry, String> {
    let result = tauri::async_runtime::spawn_blocking(move || {
        profile_service::import_profile_zip(&app, &args.zip_path)
    })
    .await
    .map_err(|e| format!("Task failed: {e}"))?;

    result.map_err(|e| e.to_string())
}
