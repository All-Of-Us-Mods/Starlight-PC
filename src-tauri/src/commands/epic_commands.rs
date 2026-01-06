use crate::utils::epic_api::{self, EpicApi};
use log::{debug, error, info, warn};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tauri::{Emitter, Manager, WebviewUrl, WebviewWindowBuilder, webview::PageLoadEvent};

const EPIC_LOGIN_WINDOW: &str = "epic-login";
const EPIC_REDIRECT_PATH: &str = "/id/api/redirect";
const CALLBACK_SCHEME: &str = "starlight";

/// JavaScript injected into the redirect page to extract the auth code.
/// Navigates to our custom scheme URL which we intercept in `on_navigation`.
const EXTRACT_CODE_JS: &str = r#"
(function() {
    if (window.__STARLIGHT_EXTRACTED__) return;
    window.__STARLIGHT_EXTRACTED__ = true;
    try {
        const json = JSON.parse(document.body.innerText);
        if (json.authorizationCode) {
            location.href = 'starlight://auth?code=' + encodeURIComponent(json.authorizationCode);
        }
    } catch(e) {}
})();
"#;

// ─── Commands ────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn get_epic_auth_url() -> String {
    EpicApi::get_auth_url()
}

#[tauri::command]
pub async fn epic_login_with_code(code: String) -> Result<(), String> {
    info!("Epic login with manual code");
    do_login(&code).await
}

#[tauri::command]
pub async fn epic_login_with_webview(app: tauri::AppHandle) -> Result<(), String> {
    // Focus existing window if already open
    if let Some(win) = app.get_webview_window(EPIC_LOGIN_WINDOW) {
        let _ = win.set_focus();
        return Ok(());
    }

    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("epic_webview");

    let handled = Arc::new(AtomicBool::new(false));

    build_login_window(&app, &data_dir, handled)?;

    let _ = app.emit("epic-login-started", ());
    Ok(())
}

#[tauri::command]
pub async fn epic_try_restore_session() -> Result<bool, String> {
    let Some(saved) = epic_api::load_session() else {
        return Ok(false);
    };

    match EpicApi::new()?.refresh_session(&saved.refresh_token).await {
        Ok(session) => {
            epic_api::save_session(&session)?;
            info!("Epic session restored");
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
    info!("Epic logout");
    epic_api::clear_session()
}

#[tauri::command]
pub async fn epic_is_logged_in() -> Result<bool, String> {
    Ok(epic_api::load_session().is_some())
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

async fn do_login(code: &str) -> Result<(), String> {
    let code = code.trim().replace('"', "");
    let session = EpicApi::new()?.login_with_auth_code(&code).await?;
    epic_api::save_session(&session)?;
    info!("Epic login successful");
    Ok(())
}

fn build_login_window(
    app: &tauri::AppHandle,
    data_dir: &std::path::Path,
    handled: Arc<AtomicBool>,
) -> Result<(), String> {
    let auth_url: url::Url = EpicApi::get_auth_url()
        .parse()
        .map_err(|e| format!("Invalid auth URL: {e}"))?;

    let app_nav = app.clone();
    let app_close = app.clone();
    let handled_nav = handled.clone();

    let window = WebviewWindowBuilder::new(app, EPIC_LOGIN_WINDOW, WebviewUrl::External(auth_url))
        .title("Login to Epic Games")
        .inner_size(500.0, 700.0)
        .center()
        .resizable(true)
        .data_directory(data_dir.to_path_buf())
        .on_page_load(|webview, payload| {
            if payload.event() == PageLoadEvent::Finished
                && let Ok(url) = webview.url()
                && url.path() == EPIC_REDIRECT_PATH
            {
                debug!("Epic redirect page loaded");
                let _ = webview.eval(EXTRACT_CODE_JS);
            }
        })
        .on_navigation(move |url| {
            if url.scheme() != CALLBACK_SCHEME {
                return true;
            }

            // Prevent duplicate handling
            // Prevent duplicate handling
            if handled_nav.swap(true, Ordering::SeqCst) {
                return false;
            }

            let app = app_nav.clone();
            if let Some(code) = extract_code_param(url) {
                tauri::async_runtime::spawn(async move {
                    handle_auth_result(&app, do_login(&code).await);
                    close_login_window(&app);
                });
            } else {
                let _ = app.emit("epic-login-error", "Missing authorization code".to_string());
                close_login_window(&app);
            }
            false
        })
        .build()
        .map_err(|e| format!("Failed to create window: {e}"))?;

    window.on_window_event(move |event| {
        if matches!(event, tauri::WindowEvent::CloseRequested { .. })
            && !handled.load(Ordering::SeqCst)
        {
            let _ = app_close.emit("epic-login-cancelled", ());
        }
    });

    Ok(())
}

fn extract_code_param(url: &url::Url) -> Option<String> {
    url.query_pairs()
        .find(|(k, _)| k == "code")
        .map(|(_, v)| v.into_owned())
}

fn handle_auth_result(app: &tauri::AppHandle, result: Result<(), String>) {
    match result {
        Ok(()) => {
            let _ = app.emit("epic-login-success", ());
        }
        Err(e) => {
            error!("Epic login failed: {}", e);
            let _ = app.emit("epic-login-error", e);
        }
    }
}

fn close_login_window(app: &tauri::AppHandle) {
    if let Some(win) = app.get_webview_window(EPIC_LOGIN_WINDOW) {
        let _ = win.close();
    }
}
