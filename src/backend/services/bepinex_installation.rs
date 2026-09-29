//! Prepare a complete runtime before replacing any installed files.

use super::bepinex_runtime::BepInExRuntime;
use crate::backend::binary::BinaryArch;
use crate::backend::error::{AppError, AppResult};
use std::fs;
use std::path::{Path, PathBuf};

pub(super) struct StagedRuntime {
    root: PathBuf,
    workspace: PathBuf,
}

impl StagedRuntime {
    pub fn new(root: &Path) -> AppResult<Self> {
        let parent = root
            .parent()
            .ok_or_else(|| AppError::validation("Invalid profile path"))?;
        let stage = Self {
            root: root.to_path_buf(),
            workspace: parent.join(format!(".bepinex-install-{}", uuid::Uuid::new_v4())),
        };
        fs::create_dir_all(stage.contents())?;
        Ok(stage)
    }

    pub fn contents(&self) -> PathBuf {
        self.workspace.join("contents")
    }

    pub fn install(&self, architecture: BinaryArch) -> AppResult<()> {
        BepInExRuntime::new(&self.contents()).validate(architecture)?;
        self.promote(Path::new(""))
    }

    fn promote(&self, relative: &Path) -> AppResult<()> {
        let source = self.contents().join(relative);
        let destination = self.root.join(relative);
        // Replace the native runtime and core as units to remove old arch-only
        // files. Merge the rest of the package, preserving user plugins/config.
        if source.is_dir()
            && !["dotnet", "BepInEx/core"]
                .iter()
                .any(|path| relative == Path::new(path))
        {
            fs::create_dir_all(&destination)?;
            for entry in fs::read_dir(source)? {
                self.promote(&relative.join(entry?.file_name()))?;
            }
        } else if !(["BepInEx/plugins", "BepInEx/config"]
            .iter()
            .any(|path| relative.starts_with(path))
            && destination.exists())
        {
            if destination.is_dir() {
                fs::remove_dir_all(&destination)?;
            }
            fs::rename(source, destination)?;
        }
        Ok(())
    }
}

impl Drop for StagedRuntime {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.workspace)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            log::warn!(
                "Failed to remove runtime staging directory {}: {error}",
                self.workspace.display()
            );
        }
    }
}
