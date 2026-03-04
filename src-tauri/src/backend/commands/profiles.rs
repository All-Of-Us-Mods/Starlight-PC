use crate::backend::services::profile_service::{
    self, ProfileEntry, ProfileIconSelection, UnifiedMod,
};
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
    pub profile_path: String,
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

#[tauri::command]
pub async fn profiles_get_dir<R: Runtime>(app: AppHandle<R>) -> Result<String, String> {
    profile_service::get_profiles_dir(&app).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn profiles_list<R: Runtime>(app: AppHandle<R>) -> Result<Vec<ProfileEntry>, String> {
    profile_service::get_profiles(&app).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn profiles_get_by_id<R: Runtime>(
    app: AppHandle<R>,
    args: ProfilesGetByIdArgs,
) -> Result<Option<ProfileEntry>, String> {
    profile_service::get_profile_by_id(&app, &args.id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn profiles_create<R: Runtime>(
    app: AppHandle<R>,
    args: ProfilesCreateArgs,
) -> Result<ProfileEntry, String> {
    profile_service::create_profile(&app, &args.name).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn profiles_install_bepinex<R: Runtime>(
    app: AppHandle<R>,
    args: ProfilesInstallBepInExArgs,
) -> Result<(), String> {
    profile_service::install_bepinex_for_profile(
        app,
        profile_service::ProfileInstallArgs {
            profile_id: args.profile_id,
            profile_path: args.profile_path,
        },
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn profiles_delete<R: Runtime>(
    app: AppHandle<R>,
    args: ProfilesDeleteArgs,
) -> Result<(), String> {
    profile_service::delete_profile(&app, &args.profile_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn profiles_rename<R: Runtime>(
    app: AppHandle<R>,
    args: ProfilesRenameArgs,
) -> Result<(), String> {
    profile_service::rename_profile(&app, &args.profile_id, &args.new_name)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn profiles_update_icon<R: Runtime>(
    app: AppHandle<R>,
    args: ProfilesUpdateIconArgs,
) -> Result<(), String> {
    profile_service::update_profile_icon(&app, &args.profile_id, args.selection)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn profiles_get_active<R: Runtime>(
    app: AppHandle<R>,
) -> Result<Option<ProfileEntry>, String> {
    profile_service::get_active_profile(&app).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn profiles_update_last_launched<R: Runtime>(
    app: AppHandle<R>,
    args: ProfilesUpdateLastLaunchedArgs,
) -> Result<(), String> {
    profile_service::update_last_launched(&app, &args.profile_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn profiles_add_mod<R: Runtime>(
    app: AppHandle<R>,
    args: ProfilesAddModArgs,
) -> Result<(), String> {
    profile_service::add_mod_to_profile(
        &app,
        &args.profile_id,
        &args.mod_id,
        &args.version,
        &args.file,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn profiles_add_play_time<R: Runtime>(
    app: AppHandle<R>,
    args: ProfilesAddPlayTimeArgs,
) -> Result<(), String> {
    profile_service::add_play_time(&app, &args.profile_id, args.duration_ms)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn profiles_remove_mod<R: Runtime>(
    app: AppHandle<R>,
    args: ProfilesRemoveModArgs,
) -> Result<(), String> {
    profile_service::remove_mod_from_profile(&app, &args.profile_id, &args.mod_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn profiles_get_mod_files(args: ProfilesModFilesArgs) -> Result<Vec<String>, String> {
    Ok(profile_service::get_mod_files(&args.profile_path))
}

#[tauri::command]
pub async fn profiles_delete_mod_file(args: ProfilesDeleteModFileArgs) -> Result<(), String> {
    profile_service::delete_mod_file(&args.profile_path, &args.file_name).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn profiles_get_log(args: ProfilesGetLogArgs) -> Result<String, String> {
    Ok(profile_service::get_profile_log(
        &args.profile_path,
        args.file_name.as_deref().unwrap_or("LogOutput.log"),
    ))
}

#[tauri::command]
pub async fn profiles_read_binary_file(
    args: ProfilesReadBinaryFileArgs,
) -> Result<Vec<u8>, String> {
    profile_service::read_binary_file(&args.path).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn profiles_delete_unified_mod<R: Runtime>(
    app: AppHandle<R>,
    args: ProfilesDeleteUnifiedModArgs,
) -> Result<(), String> {
    profile_service::delete_unified_mod(&app, &args.profile_id, args.mod_entry)
        .map_err(|e| e.to_string())
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
    let result =
        tauri::async_runtime::spawn_blocking(move || profile_service::import_profile_zip(&app, &args.zip_path))
            .await
            .map_err(|e| format!("Task failed: {e}"))?;

    result.map_err(|e| e.to_string())
}
