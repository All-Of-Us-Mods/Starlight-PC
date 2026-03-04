use crate::backend::error::AppResult;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager, Runtime};

const DEFAULT_BEPINEX_URL: &str = "https://builds.bepinex.dev/projects/bepinex_be/752/BepInEx-Unity.IL2CPP-win-x86-6.0.0-be.752%2Bdd0655f.zip";
const SETTINGS_FILE_NAME: &str = "settings.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GamePlatform {
    Steam,
    Epic,
    Xbox,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub bepinex_url: String,
    pub among_us_path: String,
    pub close_on_launch: bool,
    pub allow_multi_instance_launch: bool,
    pub game_platform: GamePlatform,
    pub cache_bepinex: bool,
    pub xbox_app_id: Option<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            bepinex_url: DEFAULT_BEPINEX_URL.to_string(),
            among_us_path: String::new(),
            close_on_launch: false,
            allow_multi_instance_launch: false,
            game_platform: GamePlatform::Steam,
            cache_bepinex: false,
            xbox_app_id: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct AppSettingsPatch {
    pub bepinex_url: Option<String>,
    pub among_us_path: Option<String>,
    pub close_on_launch: Option<bool>,
    pub allow_multi_instance_launch: Option<bool>,
    pub game_platform: Option<GamePlatform>,
    pub cache_bepinex: Option<bool>,
    pub xbox_app_id: Option<String>,
}

fn app_data_dir<R: Runtime>(app: &AppHandle<R>) -> AppResult<PathBuf> {
    app.path().app_data_dir().map_err(Into::into)
}

fn settings_path<R: Runtime>(app: &AppHandle<R>) -> AppResult<PathBuf> {
    Ok(app_data_dir(app)?.join(SETTINGS_FILE_NAME))
}

fn bepinex_cache_path<R: Runtime>(app: &AppHandle<R>) -> AppResult<String> {
    Ok(app_data_dir(app)?
        .join("cache")
        .join("bepinex.zip")
        .to_string_lossy()
        .to_string())
}

pub fn get_settings<R: Runtime>(app: &AppHandle<R>) -> AppResult<AppSettings> {
    let path = settings_path(app)?;
    if !path.exists() {
        return Ok(AppSettings::default());
    }

    let raw = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&raw).unwrap_or_default())
}

pub fn update_settings<R: Runtime>(
    app: &AppHandle<R>,
    patch: AppSettingsPatch,
) -> AppResult<AppSettings> {
    let mut settings = get_settings(app)?;

    if let Some(value) = patch.bepinex_url {
        settings.bepinex_url = value;
    }
    if let Some(value) = patch.among_us_path {
        settings.among_us_path = value;
    }
    if let Some(value) = patch.close_on_launch {
        settings.close_on_launch = value;
    }
    if let Some(value) = patch.allow_multi_instance_launch {
        settings.allow_multi_instance_launch = value;
    }
    if let Some(value) = patch.game_platform {
        settings.game_platform = value;
    }
    if let Some(value) = patch.cache_bepinex {
        settings.cache_bepinex = value;
    }
    if patch.xbox_app_id.is_some() {
        settings.xbox_app_id = patch.xbox_app_id;
    }

    let path = settings_path(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(&settings)?)?;

    Ok(settings)
}

pub fn get_bepinex_cache_path<R: Runtime>(app: &AppHandle<R>) -> AppResult<String> {
    bepinex_cache_path(app)
}

pub fn get_app_data_dir<R: Runtime>(app: &AppHandle<R>) -> AppResult<String> {
    Ok(app_data_dir(app)?.to_string_lossy().to_string())
}

pub fn auto_detect_bepinex_architecture<R: Runtime>(
    app: &AppHandle<R>,
    game_path: &str,
) -> AppResult<Option<String>> {
    let mut settings = get_settings(app)?;
    let crash_handler_path = PathBuf::from(game_path).join("UnityCrashHandler64.exe");
    let is_64_bit = crash_handler_path.exists();

    let updated_url = if is_64_bit {
        settings.bepinex_url.replace("win-x86-", "win-x64-")
    } else {
        settings.bepinex_url.replace("win-x64-", "win-x86-")
    };

    if updated_url == settings.bepinex_url {
        return Ok(None);
    }

    settings.bepinex_url = updated_url.clone();
    let path = settings_path(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(&settings)?)?;

    Ok(Some(updated_url))
}

pub async fn api_get_json(api_base_url: &str, path: &str) -> AppResult<serde_json::Value> {
    let base = api_base_url.trim_end_matches('/');
    let route = if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    };
    let url = format!("{base}{route}");
    let response = reqwest::get(url).await?;
    let status = response.status();
    if !status.is_success() {
        return Err(crate::backend::error::AppError::other(format!(
            "HTTP error: {}",
            status
        )));
    }
    Ok(response.json::<serde_json::Value>().await?)
}
