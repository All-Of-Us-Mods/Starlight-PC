use crate::utils::epic_api::{self, EpicApi};
use log::{debug, error, info, warn};

#[tauri::command]
pub fn get_epic_auth_url() -> String {
    let url = EpicApi::get_auth_url();
    debug!("Generated Epic auth URL");
    url
}

#[tauri::command]
pub async fn epic_login_with_code(code: String) -> Result<(), String> {
    info!("Attempting Epic login with authorization code");
    let code = code.trim().replace('"', "");
    let api = EpicApi::new()?;
    let session = api.login_with_auth_code(&code).await.map_err(|e| {
        error!("Epic login failed: {}", e);
        e
    })?;
    epic_api::save_session(&session)?;
    info!("Epic login successful");
    Ok(())
}

#[tauri::command]
pub async fn epic_try_restore_session() -> Result<bool, String> {
    debug!("Attempting to restore Epic session");
    let Some(saved) = epic_api::load_session() else {
        debug!("No saved Epic session found");
        return Ok(false);
    };

    let api = EpicApi::new()?;
    match api.refresh_session(&saved.refresh_token).await {
        Ok(session) => {
            epic_api::save_session(&session)?;
            info!("Epic session restored successfully");
            Ok(true)
        }
        Err(e) => {
            warn!("Failed to restore Epic session: {}", e);
            Ok(false)
        }
    }
}

#[tauri::command]
pub async fn epic_logout() -> Result<(), String> {
    info!("Logging out of Epic Games");
    epic_api::clear_session()
}

#[tauri::command]
pub async fn epic_is_logged_in() -> Result<bool, String> {
    let logged_in = epic_api::load_session().is_some();
    debug!("Epic logged in status: {}", logged_in);
    Ok(logged_in)
}
