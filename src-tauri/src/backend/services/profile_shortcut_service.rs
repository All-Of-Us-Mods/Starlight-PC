use crate::backend::error::{AppError, AppResult};
#[cfg(windows)]
use crate::backend::services::profile_service;
#[cfg(windows)]
use image::{ImageFormat, imageops::FilterType};
#[cfg(windows)]
use std::fs;
#[cfg(windows)]
use std::path::PathBuf;
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
const SHORTCUT_ICON_DIR: &str = "shortcut-icons";
#[cfg(windows)]
const SHORTCUT_ICON_SIZE: u32 = 256;

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

#[cfg(windows)]
fn shortcut_icon_path<R: Runtime>(app: &AppHandle<R>, profile_id: &str) -> AppResult<PathBuf> {
    let icon_dir = app.path().app_data_dir()?.join(SHORTCUT_ICON_DIR);
    fs::create_dir_all(&icon_dir)?;
    Ok(icon_dir.join(format!("{profile_id}.ico")))
}

#[cfg(windows)]
fn write_shortcut_icon<R: Runtime>(
    app: &AppHandle<R>,
    profile_id: &str,
    icon_bytes: &[u8],
) -> AppResult<PathBuf> {
    let image = image::load_from_memory(icon_bytes).map_err(|error| {
        AppError::validation(format!("Failed to decode profile icon image: {error}"))
    })?;

    let resized =
        image.resize_to_fill(SHORTCUT_ICON_SIZE, SHORTCUT_ICON_SIZE, FilterType::Lanczos3);
    let icon_path = shortcut_icon_path(app, profile_id)?;
    resized
        .save_with_format(&icon_path, ImageFormat::Ico)
        .map_err(|error| AppError::platform(format!("Failed to write shortcut icon: {error}")))?;
    Ok(icon_path)
}

#[cfg(windows)]
fn resolve_icon_path<R: Runtime>(
    app: &AppHandle<R>,
    profile_id: &str,
    icon_bytes: Option<&[u8]>,
) -> AppResult<PathBuf> {
    let default_icon_path = std::env::current_exe()?;

    let Some(icon_bytes) = icon_bytes.filter(|bytes| !bytes.is_empty()) else {
        return Ok(default_icon_path);
    };

    match write_shortcut_icon(app, profile_id, icon_bytes) {
        Ok(path) => Ok(path),
        Err(error) => {
            log::warn!(
                "Failed to create profile shortcut icon for '{}': {}",
                profile_id,
                error
            );
            Ok(default_icon_path)
        }
    }
}

pub fn create_desktop_shortcut<R: Runtime>(
    app: &AppHandle<R>,
    profile_id: &str,
    icon_bytes: Option<&[u8]>,
) -> AppResult<String> {
    #[cfg(not(windows))]
    {
        let _ = (app, profile_id, icon_bytes);
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
        let icon_path = resolve_icon_path(app, &profile.id, icon_bytes)?;
        let shortcut_contents = format!(
            "[InternetShortcut]\r\nURL={shortcut_url}\r\nIconFile={}\r\nIconIndex=0\r\n",
            icon_path.to_string_lossy()
        );

        fs::write(&shortcut_path, shortcut_contents)?;

        Ok(shortcut_path.to_string_lossy().to_string())
    }
}
