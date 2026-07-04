use crate::backend::directories;
use crate::backend::error::AppResult;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const DEFAULT_BEPINEX_URL_X86: &str = "https://builds.bepinex.dev/projects/bepinex_be/752/BepInEx-Unity.IL2CPP-win-x86-6.0.0-be.752%2Bdd0655f.zip";
const DEFAULT_BEPINEX_URL_X64: &str = "https://builds.bepinex.dev/projects/bepinex_be/752/BepInEx-Unity.IL2CPP-win-x64-6.0.0-be.752%2Bdd0655f.zip";
const SETTINGS_FILE_NAME: &str = "settings.json";

/// Default seconds to wait between queued multi-instance launches so the first
/// instance can warm the shared BepInEx cache/interop before the next starts.
pub const DEFAULT_MULTI_INSTANCE_LAUNCH_DELAY_SECS: u64 = 10;

fn default_multi_instance_launch_delay_secs() -> u64 {
    DEFAULT_MULTI_INSTANCE_LAUNCH_DELAY_SECS
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GamePlatform {
    Steam,
    Epic,
    Xbox,
}

/// Background tint family for the app UI. The palettes live in `crate::theme`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppTint {
    /// Pure black backgrounds.
    #[default]
    Black,
    /// Warm brown-tinted near-black, like the upstream app.
    Warm,
    /// Cool zinc near-black (the pre-theming look).
    Zinc,
    /// Red-tinted darks.
    Crimson,
    /// Purple-tinted darks.
    Violet,
}

/// Accent color for the app UI, independent of the background tint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccentColor {
    /// Starlight gold.
    #[default]
    Starlight,
    /// Blue (the pre-theming accent).
    Blue,
    /// Crewmate red.
    Red,
    /// Impostor purple.
    Purple,
    /// Green.
    Green,
}

fn default_true() -> bool {
    true
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LinuxRunnerKind {
    Wine,
    Proton,
    /// Hand the launch to the Steam client (`steam -applaunch`) so Steamworks
    /// (online) and the Steam Linux Runtime (audio) are set up by Steam itself.
    #[default]
    Steam,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub bepinex_url_x86: String,
    pub bepinex_url_x64: String,
    pub among_us_path: String,
    pub close_on_launch: bool,
    pub allow_multi_instance_launch: bool,
    #[serde(default = "default_multi_instance_launch_delay_secs")]
    pub multi_instance_launch_delay_secs: u64,
    pub game_platform: GamePlatform,
    pub cache_bepinex: bool,
    pub xbox_app_id: Option<String>,
    #[serde(default)]
    pub linux_runner_kind: LinuxRunnerKind,
    #[serde(default)]
    pub linux_runner_binary: String,
    #[serde(default)]
    pub linux_wine_prefix: String,
    #[serde(default)]
    pub linux_proton_compat_data_path: String,
    #[serde(default)]
    pub linux_proton_steam_client_path: String,
    #[serde(default)]
    pub linux_proton_use_steam_run: bool,
    #[serde(default)]
    pub app_tint: AppTint,
    #[serde(default)]
    pub accent_color: AccentColor,
    #[serde(default = "default_true")]
    pub show_stars_background: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            bepinex_url_x86: DEFAULT_BEPINEX_URL_X86.to_string(),
            bepinex_url_x64: DEFAULT_BEPINEX_URL_X64.to_string(),
            among_us_path: String::new(),
            close_on_launch: false,
            allow_multi_instance_launch: false,
            multi_instance_launch_delay_secs: DEFAULT_MULTI_INSTANCE_LAUNCH_DELAY_SECS,
            game_platform: GamePlatform::Steam,
            cache_bepinex: false,
            xbox_app_id: None,
            linux_runner_kind: LinuxRunnerKind::Steam,
            linux_runner_binary: String::new(),
            linux_wine_prefix: String::new(),
            linux_proton_compat_data_path: String::new(),
            linux_proton_steam_client_path: String::new(),
            linux_proton_use_steam_run: true,
            app_tint: AppTint::default(),
            accent_color: AccentColor::default(),
            show_stars_background: true,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct AppSettingsPatch {
    pub bepinex_url_x86: Option<String>,
    pub bepinex_url_x64: Option<String>,
    pub among_us_path: Option<String>,
    pub close_on_launch: Option<bool>,
    pub allow_multi_instance_launch: Option<bool>,
    pub multi_instance_launch_delay_secs: Option<u64>,
    pub game_platform: Option<GamePlatform>,
    pub cache_bepinex: Option<bool>,
    pub xbox_app_id: Option<Option<String>>,
    pub linux_runner_kind: Option<LinuxRunnerKind>,
    pub linux_runner_binary: Option<String>,
    pub linux_wine_prefix: Option<String>,
    pub linux_proton_compat_data_path: Option<String>,
    pub linux_proton_steam_client_path: Option<String>,
    pub linux_proton_use_steam_run: Option<bool>,
    pub app_tint: Option<AppTint>,
    pub accent_color: Option<AccentColor>,
    pub show_stars_background: Option<bool>,
}

fn settings_path() -> AppResult<PathBuf> {
    Ok(directories::app_data_dir()?.join(SETTINGS_FILE_NAME))
}

fn write_settings_to_file(path: &Path, settings: &AppSettings) -> AppResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temporary_path = path.with_extension("json.tmp");
    fs::write(&temporary_path, serde_json::to_vec_pretty(settings)?)?;
    fs::rename(&temporary_path, path)?;
    Ok(())
}

fn read_legacy_settings() -> AppResult<Option<AppSettings>> {
    let registry_path = directories::app_data_dir()?
        .join(".settings")
        .join("registry.json");
    if !registry_path.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(&registry_path)?;

    let Ok(store) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return Ok(None);
    };

    let Some(settings_val) = store.get("settings") else {
        return Ok(None);
    };

    #[derive(Deserialize)]
    struct LegacySettingsPatch {
        bepinex_url: Option<String>,
        bepinex_url_x86: Option<String>,
        bepinex_url_x64: Option<String>,
        among_us_path: Option<String>,
        close_on_launch: Option<bool>,
        allow_multi_instance_launch: Option<bool>,
        game_platform: Option<GamePlatform>,
        cache_bepinex: Option<bool>,
        xbox_app_id: Option<String>,
        linux_runner_kind: Option<LinuxRunnerKind>,
        linux_runner_binary: Option<String>,
        linux_wine_prefix: Option<String>,
        linux_proton_compat_data_path: Option<String>,
        linux_proton_steam_client_path: Option<String>,
        linux_proton_use_steam_run: Option<bool>,
    }

    let mut settings = AppSettings::default();
    let Ok(patch) = serde_json::from_value::<LegacySettingsPatch>(settings_val.clone()) else {
        return Ok(None);
    };

    if let Some(value) = patch.bepinex_url_x86 {
        settings.bepinex_url_x86 = value;
    }
    if let Some(value) = patch.bepinex_url_x64 {
        settings.bepinex_url_x64 = value;
    }
    if let Some(value) = patch.bepinex_url {
        settings.bepinex_url_x86 = value.clone();
        settings.bepinex_url_x64 = value.replace("win-x86-", "win-x64-");
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
    if let Some(value) = patch.linux_runner_kind {
        settings.linux_runner_kind = value;
    }
    if let Some(value) = patch.linux_runner_binary {
        settings.linux_runner_binary = value;
    }
    if let Some(value) = patch.linux_wine_prefix {
        settings.linux_wine_prefix = value;
    }
    if let Some(value) = patch.linux_proton_compat_data_path {
        settings.linux_proton_compat_data_path = value;
    }
    if let Some(value) = patch.linux_proton_steam_client_path {
        settings.linux_proton_steam_client_path = value;
    }
    if let Some(value) = patch.linux_proton_use_steam_run {
        settings.linux_proton_use_steam_run = value;
    }

    Ok(Some(settings))
}

pub fn get_settings() -> AppResult<AppSettings> {
    let path = settings_path()?;
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

    if let Some(legacy_settings) = read_legacy_settings()? {
        write_settings_to_file(&path, &legacy_settings)?;
        return Ok(legacy_settings);
    }

    Ok(AppSettings::default())
}

pub fn update_settings(patch: AppSettingsPatch) -> AppResult<AppSettings> {
    let mut settings = get_settings()?;

    if let Some(value) = patch.bepinex_url_x86 {
        settings.bepinex_url_x86 = value;
    }
    if let Some(value) = patch.bepinex_url_x64 {
        settings.bepinex_url_x64 = value;
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
    if let Some(value) = patch.multi_instance_launch_delay_secs {
        settings.multi_instance_launch_delay_secs = value;
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
    if let Some(value) = patch.linux_runner_kind {
        settings.linux_runner_kind = value;
    }
    if let Some(value) = patch.linux_runner_binary {
        settings.linux_runner_binary = value;
    }
    if let Some(value) = patch.linux_wine_prefix {
        settings.linux_wine_prefix = value;
    }
    if let Some(value) = patch.linux_proton_compat_data_path {
        settings.linux_proton_compat_data_path = value;
    }
    if let Some(value) = patch.linux_proton_steam_client_path {
        settings.linux_proton_steam_client_path = value;
    }
    if let Some(value) = patch.linux_proton_use_steam_run {
        settings.linux_proton_use_steam_run = value;
    }
    if let Some(value) = patch.app_tint {
        settings.app_tint = value;
    }
    if let Some(value) = patch.accent_color {
        settings.accent_color = value;
    }
    if let Some(value) = patch.show_stars_background {
        settings.show_stars_background = value;
    }

    let path = settings_path()?;
    write_settings_to_file(&path, &settings)?;

    Ok(settings)
}

pub fn get_bepinex_cache_path(architecture: &str) -> AppResult<String> {
    Ok(directories::app_data_dir()?
        .join("cache")
        .join(format!("bepinex-{architecture}.zip"))
        .to_string_lossy()
        .to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_settings_to_file_round_trips_and_leaves_no_temp_file() {
        let path = std::env::temp_dir().join(format!(
            "starlight-settings-test-{}.json",
            std::process::id()
        ));

        write_settings_to_file(&path, &AppSettings::default()).unwrap();

        let raw = fs::read_to_string(&path).unwrap();
        serde_json::from_str::<AppSettings>(&raw).unwrap();
        assert!(!path.with_extension("json.tmp").exists());

        fs::remove_file(&path).unwrap();
    }
}
