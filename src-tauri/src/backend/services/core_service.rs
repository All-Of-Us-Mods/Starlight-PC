use crate::backend::error::AppResult;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_store::StoreExt;

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
    pub xbox_app_id: Option<Option<String>>,
}

fn app_data_dir<R: Runtime>(app: &AppHandle<R>) -> AppResult<PathBuf> {
    app.path().app_data_dir().map_err(Into::into)
}

fn settings_path<R: Runtime>(app: &AppHandle<R>) -> AppResult<PathBuf> {
    Ok(app_data_dir(app)?.join(SETTINGS_FILE_NAME))
}

fn write_settings_to_file(path: &PathBuf, settings: &AppSettings) -> AppResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(settings)?)?;
    Ok(())
}

fn read_legacy_settings<R: Runtime>(app: &AppHandle<R>) -> AppResult<Option<AppSettings>> {
    let store = app.store("registry.json").map_err(|e| {
        crate::backend::error::AppError::state(format!("Failed to load registry store: {e}"))
    })?;

    let Some(raw) = store.get("settings") else {
        return Ok(None);
    };

    #[derive(Deserialize)]
    struct LegacySettingsPatch {
        bepinex_url: Option<String>,
        among_us_path: Option<String>,
        close_on_launch: Option<bool>,
        allow_multi_instance_launch: Option<bool>,
        game_platform: Option<GamePlatform>,
        cache_bepinex: Option<bool>,
        xbox_app_id: Option<String>,
    }

    let mut settings = AppSettings::default();
    let Ok(patch) = serde_json::from_value::<LegacySettingsPatch>(raw) else {
        return Ok(None);
    };

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
    if let Some(value) = patch.xbox_app_id {
        settings.xbox_app_id = Some(value);
    }

    Ok(Some(settings))
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
    if path.exists() {
        let raw = fs::read_to_string(&path)?;
        match serde_json::from_str::<AppSettings>(&raw) {
            Ok(settings) => return Ok(settings),
            Err(error) => {
                log::warn!(
                    "Failed to parse settings file at '{}': {error}. Falling back to migration/default settings.",
                    path.display()
                );
            }
        }
    }

    if let Some(legacy_settings) = read_legacy_settings(app)? {
        write_settings_to_file(&path, &legacy_settings)?;
        return Ok(legacy_settings);
    }

    Ok(AppSettings::default())
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
    if let Some(value) = patch.xbox_app_id {
        settings.xbox_app_id = value;
    }

    let path = settings_path(app)?;
    write_settings_to_file(&path, &settings)?;

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
    write_settings_to_file(&path, &settings)?;

    Ok(Some(updated_url))
}
