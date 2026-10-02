//! Game installations: where Among Us lives and how to launch it. The settings'
//! top-level installation is the default; a profile may select a linked one.

use super::core_service::{AppSettings, GamePlatform, LinuxRunnerKind};
use super::profile_service::ProfileEntry;
use crate::backend::binary::{BinaryArch, read_pe_arch};
use crate::backend::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const GAME_EXE_NAME: &str = "Among Us.exe";

/// One copy of the game and the launcher configuration it needs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameSetup {
    pub among_us_path: String,
    pub game_platform: GamePlatform,
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
}

impl Default for GameSetup {
    fn default() -> Self {
        Self {
            among_us_path: String::new(),
            game_platform: GamePlatform::Steam,
            linux_runner_kind: LinuxRunnerKind::Steam,
            linux_runner_binary: String::new(),
            linux_wine_prefix: String::new(),
            linux_proton_compat_data_path: String::new(),
            linux_proton_steam_client_path: String::new(),
            linux_proton_use_steam_run: true,
        }
    }
}

impl GameSetup {
    pub fn game_dir(&self) -> &Path {
        Path::new(self.among_us_path.trim())
    }

    /// Validate the game location before either vanilla or modded launches.
    pub fn executable(&self) -> AppResult<PathBuf> {
        if self.among_us_path.trim().is_empty() {
            return Err(AppError::validation(
                "Among Us path is not set. Configure it in Settings.",
            ));
        }
        let exe = self.game_dir().join(GAME_EXE_NAME);
        if !exe.is_file() {
            return Err(AppError::validation(format!(
                "{GAME_EXE_NAME} not found at {}",
                exe.display()
            )));
        }
        Ok(exe)
    }

    /// Which BepInEx build this game needs, read from its executable: stores
    /// have switched bitness between game updates, so the binary is the only
    /// reliable source. Falls back to x64 (what most stores ship).
    pub fn arch(&self) -> BinaryArch {
        read_pe_arch(&self.game_dir().join(GAME_EXE_NAME)).unwrap_or(BinaryArch::X64)
    }

    pub fn label(&self) -> String {
        format!(
            "{} ({})",
            self.game_platform.display_name(),
            self.among_us_path
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameInstallation {
    pub id: String,
    #[serde(flatten)]
    pub setup: GameSetup,
}

impl AppSettings {
    /// `None` selects the default installation.
    pub fn installation(&self, id: Option<&str>) -> AppResult<&GameSetup> {
        let Some(id) = id else {
            return Ok(&self.game);
        };
        self.game_installations
            .iter()
            .find(|installation| installation.id == id)
            .map(|installation| &installation.setup)
            .ok_or_else(|| {
                AppError::validation(
                    "The linked installation was removed. Choose an installation for this profile.",
                )
            })
    }

    /// Link the default installation, or refresh the link to the same folder.
    pub fn link_current_installation(&mut self) -> AppResult<()> {
        self.game.executable()?;
        let path = self.game.game_dir().canonicalize()?;
        let mut setup = self.game.clone();
        setup.among_us_path = self.game.among_us_path.trim().to_string();
        match self
            .game_installations
            .iter_mut()
            .find(|i| i.setup.game_dir().canonicalize().ok().as_ref() == Some(&path))
        {
            Some(existing) => existing.setup = setup,
            None => self.game_installations.push(GameInstallation {
                id: uuid::Uuid::new_v4().to_string(),
                setup,
            }),
        }
        Ok(())
    }

    /// An installation must be unused before it can be unlinked, so profile
    /// selections stay valid without rewriting any profile.
    pub fn unlink_installation(&mut self, id: &str, profiles: &[ProfileEntry]) -> AppResult<()> {
        let users: Vec<&str> = profiles
            .iter()
            .filter(|profile| profile.installation_id.as_deref() == Some(id))
            .map(|profile| profile.name.as_str())
            .collect();
        if !users.is_empty() {
            return Err(AppError::validation(format!(
                "Choose another installation for these profiles before unlinking: {}.",
                users.join(", ")
            )));
        }
        self.game_installations.retain(|i| i.id != id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::test_support::{TempDir, write_test_pe};

    fn linked(settings: &mut AppSettings, path: &str, platform: GamePlatform) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        settings.game_installations.push(GameInstallation {
            id: id.clone(),
            setup: GameSetup {
                among_us_path: path.into(),
                game_platform: platform,
                ..Default::default()
            },
        });
        id
    }

    #[test]
    fn arch_reads_the_game_executable_and_falls_back_to_x64() {
        let dir = TempDir::new("game-arch");
        let game = GameSetup {
            among_us_path: dir.0.to_string_lossy().into_owned(),
            ..Default::default()
        };
        assert_eq!(game.arch(), BinaryArch::X64);
        write_test_pe(&dir.0.join(GAME_EXE_NAME), 0x014c);
        assert_eq!(game.arch(), BinaryArch::X86);
        write_test_pe(&dir.0.join(GAME_EXE_NAME), 0x8664);
        assert_eq!(game.arch(), BinaryArch::X64);
    }

    #[test]
    fn selection_resolves_linked_or_default_installation() {
        let mut settings = AppSettings::default();
        settings.game.among_us_path = "steam".into();
        let id = linked(&mut settings, "epic", GamePlatform::Epic);

        let selected = settings.installation(Some(&id)).unwrap();
        assert_eq!(selected.among_us_path, "epic");
        assert_eq!(selected.game_platform, GamePlatform::Epic);
        assert_eq!(settings.installation(None).unwrap().among_us_path, "steam");
        assert!(settings.installation(Some("removed")).is_err());
    }

    #[test]
    fn settings_keep_their_flat_on_disk_format() {
        let mut settings = AppSettings::default();
        settings.game.among_us_path = "steam".into();
        linked(&mut settings, "epic", GamePlatform::Epic);
        let mut json = serde_json::to_value(&settings).unwrap();
        assert_eq!(json["among_us_path"], "steam");
        assert_eq!(json["game_installations"][0]["among_us_path"], "epic");

        json.as_object_mut().unwrap().remove("game_installations");
        let restored: AppSettings = serde_json::from_value(json).unwrap();
        assert!(restored.game_installations.is_empty());
        assert_eq!(restored.game.among_us_path, "steam");
    }

    #[test]
    fn linking_validates_and_updates_the_same_folder() {
        let dir = TempDir::new("link-installation");
        let mut settings = AppSettings::default();
        settings.game.among_us_path = dir.0.to_string_lossy().into_owned();
        assert!(settings.link_current_installation().is_err());

        write_test_pe(&dir.0.join(GAME_EXE_NAME), 0x8664);
        settings.link_current_installation().unwrap();
        let id = settings.game_installations[0].id.clone();
        settings.game.game_platform = GamePlatform::Epic;
        settings.link_current_installation().unwrap();
        assert_eq!(settings.game_installations.len(), 1);
        assert_eq!(settings.game_installations[0].id, id);
        assert_eq!(
            settings.game_installations[0].setup.game_platform,
            GamePlatform::Epic
        );
    }

    #[test]
    fn unlinking_a_used_installation_is_refused() {
        let mut settings = AppSettings::default();
        let id = linked(&mut settings, "epic", GamePlatform::Epic);
        let profile: ProfileEntry = serde_json::from_value(serde_json::json!({
            "installation_id": id, "id": "profile", "name": "Linked profile",
            "path": "unused", "created_at": 0, "mods": []
        }))
        .unwrap();
        assert!(settings.unlink_installation(&id, &[profile]).is_err());
        assert!(settings.installation(Some(&id)).is_ok());
        settings.unlink_installation(&id, &[]).unwrap();
        assert!(settings.game_installations.is_empty());
    }
}
