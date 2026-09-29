//! Linked game installations and resolution of the effective launch settings.
//! The legacy top-level game settings remain the default for unassigned profiles.

use super::core_service::{AppSettings, GamePlatform, LinuxRunnerKind};
use crate::backend::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const GAME_EXE_NAME: &str = "Among Us.exe";

/// A linked installation includes its launcher configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameInstallation {
    pub id: String,
    pub among_us_path: String,
    pub game_platform: GamePlatform,
    pub xbox_app_id: Option<String>,
    pub linux_runner_kind: LinuxRunnerKind,
    pub linux_runner_binary: String,
    pub linux_wine_prefix: String,
    pub linux_proton_compat_data_path: String,
    pub linux_proton_steam_client_path: String,
    pub linux_proton_use_steam_run: bool,
}

impl GameInstallation {
    pub fn from_settings(settings: &AppSettings) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            among_us_path: settings.among_us_path.clone(),
            game_platform: settings.game_platform,
            xbox_app_id: settings.xbox_app_id.clone(),
            linux_runner_kind: settings.linux_runner_kind.clone(),
            linux_runner_binary: settings.linux_runner_binary.clone(),
            linux_wine_prefix: settings.linux_wine_prefix.clone(),
            linux_proton_compat_data_path: settings.linux_proton_compat_data_path.clone(),
            linux_proton_steam_client_path: settings.linux_proton_steam_client_path.clone(),
            linux_proton_use_steam_run: settings.linux_proton_use_steam_run,
        }
    }

    pub fn label(&self) -> String {
        format!(
            "{} — {}",
            self.game_platform.display_name(),
            self.among_us_path
        )
    }

    fn apply(&self, settings: &mut AppSettings) {
        settings.among_us_path = self.among_us_path.clone();
        settings.game_platform = self.game_platform;
        settings.xbox_app_id = self.xbox_app_id.clone();
        settings.linux_runner_kind = self.linux_runner_kind.clone();
        settings.linux_runner_binary = self.linux_runner_binary.clone();
        settings.linux_wine_prefix = self.linux_wine_prefix.clone();
        settings.linux_proton_compat_data_path = self.linux_proton_compat_data_path.clone();
        settings.linux_proton_steam_client_path = self.linux_proton_steam_client_path.clone();
        settings.linux_proton_use_steam_run = self.linux_proton_use_steam_run;
    }
}

impl AppSettings {
    /// Validate the game location before either vanilla or modded launches.
    pub fn game_executable(&self) -> AppResult<PathBuf> {
        let game_path = self.among_us_path.trim();
        if game_path.is_empty() {
            return Err(AppError::validation(
                "Among Us path is not set. Configure it in Settings.",
            ));
        }

        let game_exe = PathBuf::from(game_path).join(GAME_EXE_NAME);
        if !game_exe.is_file() {
            return Err(AppError::validation(format!(
                "{GAME_EXE_NAME} not found at {}",
                game_exe.display()
            )));
        }

        Ok(game_exe)
    }

    pub fn for_installation(&self, id: Option<&str>) -> AppResult<Self> {
        let mut settings = self.clone();
        if let Some(id) = id {
            let installation = self.game_installations.iter().find(|i| i.id == id)
                .ok_or_else(|| crate::backend::error::AppError::validation(
                    "The linked installation was removed. Choose an installation for this profile."
                ))?;
            installation.apply(&mut settings);
        }
        Ok(settings)
    }

    pub fn link_current_installation(&mut self) -> AppResult<()> {
        self.game_executable()?;
        let path = Path::new(self.among_us_path.trim()).canonicalize()?;
        let mut installation = GameInstallation::from_settings(self);
        installation.among_us_path = self.among_us_path.trim().to_string();
        if let Some(existing) = self
            .game_installations
            .iter_mut()
            .find(|i| Path::new(&i.among_us_path).canonicalize().ok().as_ref() == Some(&path))
        {
            installation.id = existing.id.clone();
            *existing = installation;
        } else {
            self.game_installations.push(installation);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn linked_installation_resolves_without_changing_default() {
        let mut settings = super::AppSettings::default();
        settings.among_us_path = "steam".into();
        let mut epic = super::GameInstallation::from_settings(&settings);
        epic.among_us_path = "epic".into();
        epic.game_platform = super::GamePlatform::Epic;
        epic.linux_runner_kind = super::LinuxRunnerKind::Wine;
        epic.linux_wine_prefix = "epic-prefix".into();
        let id = epic.id.clone();
        settings.game_installations.push(epic);
        let selected = settings.for_installation(Some(&id)).unwrap();
        assert_eq!(selected.among_us_path, "epic");
        assert_eq!(selected.game_platform, super::GamePlatform::Epic);
        assert_eq!(selected.linux_wine_prefix, "epic-prefix");
        assert!(matches!(
            selected.linux_runner_kind,
            super::LinuxRunnerKind::Wine
        ));
        assert_eq!(
            settings.for_installation(None).unwrap().among_us_path,
            "steam"
        );
        assert!(settings.for_installation(Some("removed")).is_err());

        let mut old = serde_json::to_value(&settings).unwrap();
        old.as_object_mut().unwrap().remove("game_installations");
        let restored: super::AppSettings = serde_json::from_value(old).unwrap();
        assert!(restored.game_installations.is_empty());
        assert_eq!(restored.among_us_path, "steam");
    }

    #[test]
    fn linking_validates_and_updates_existing_folder() {
        let root = std::env::temp_dir().join(format!("slpc-installs-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let mut settings = super::AppSettings::default();
        settings.among_us_path = root.to_string_lossy().into_owned();
        assert!(settings.link_current_installation().is_err());
        std::fs::write(root.join("Among Us.exe"), b"test").unwrap();
        settings.link_current_installation().unwrap();
        let id = settings.game_installations[0].id.clone();
        settings.game_platform = super::GamePlatform::Epic;
        settings.link_current_installation().unwrap();
        assert_eq!(settings.game_installations.len(), 1);
        assert_eq!(settings.game_installations[0].id, id);
        assert_eq!(
            settings.game_installations[0].game_platform,
            super::GamePlatform::Epic
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
