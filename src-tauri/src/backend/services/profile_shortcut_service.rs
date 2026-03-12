use crate::backend::error::{AppError, AppResult};
#[cfg(windows)]
use crate::backend::services::profile_service;
#[cfg(windows)]
use std::fs;
#[cfg(windows)]
use tauri::Manager;
use tauri::{AppHandle, Runtime};

#[cfg(windows)]
const DEEP_LINK_SCHEME: &str = "starlight";
#[cfg(windows)]
const PROFILE_LINK_HOST: &str = "profile";
#[cfg(windows)]
const SHORTCUT_PREFIX: &str = "Starlight - ";

#[cfg(windows)]
fn sanitize_shortcut_name(name: &str) -> String {
    let sanitized = name
        .trim()
        .chars()
        .map(|ch| match ch {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '-',
            _ if ch.is_control() => '-',
            _ => ch,
        })
        .collect::<String>();

    let sanitized = sanitized
        .trim_matches(|ch: char| ch == ' ' || ch == '.')
        .to_string();

    if sanitized.is_empty() {
        "Profile".to_string()
    } else {
        sanitized
    }
}

pub fn create_desktop_shortcut<R: Runtime>(
    app: &AppHandle<R>,
    profile_id: &str,
) -> AppResult<String> {
    #[cfg(not(windows))]
    {
        let _ = (app, profile_id);
        Err(AppError::platform(
            "Desktop shortcuts are only supported on Windows",
        ))
    }

    #[cfg(windows)]
    {
        let profile = profile_service::get_profile_by_id(app, profile_id)?
            .ok_or_else(|| AppError::validation(format!("Profile '{profile_id}' not found")))?;

        let desktop_dir = app.path().desktop_dir()?;
        fs::create_dir_all(&desktop_dir)?;

        let shortcut_name = sanitize_shortcut_name(&profile.name);
        let shortcut_path = desktop_dir.join(format!("{SHORTCUT_PREFIX}{shortcut_name}.url"));
        let shortcut_url = format!("{DEEP_LINK_SCHEME}://{PROFILE_LINK_HOST}/{}", profile.id);
        let icon_path = std::env::current_exe()?;
        let shortcut_contents = format!(
            "[InternetShortcut]\r\nURL={shortcut_url}\r\nIconFile={}\r\nIconIndex=0\r\n",
            icon_path.to_string_lossy()
        );

        fs::write(&shortcut_path, shortcut_contents)?;

        Ok(shortcut_path.to_string_lossy().to_string())
    }
}
