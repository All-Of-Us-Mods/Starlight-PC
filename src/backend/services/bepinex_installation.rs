//! Prepare a complete runtime before replacing any installed files, and put
//! the previous files back if replacing them fails partway.

use super::bepinex_runtime::BepInExRuntime;
use crate::backend::binary::BinaryArch;
use crate::backend::error::{AppError, AppResult};
use std::fs;
use std::path::{Path, PathBuf};

/// Replaced as units so no files from the previous architecture survive.
const REPLACED_DIRS: [&str; 2] = ["dotnet", "BepInEx/core"];
/// The user's files; the package never overwrites what already exists here.
const PRESERVED_DIRS: [&str; 2] = ["BepInEx/plugins", "BepInEx/config"];

pub(super) struct StagedRuntime {
    root: PathBuf,
    workspace: PathBuf,
    /// Set when a rollback failed, so the backups survive for manual recovery.
    keep_workspace: bool,
}

impl StagedRuntime {
    pub fn new(root: &Path) -> AppResult<Self> {
        let parent = root
            .parent()
            .ok_or_else(|| AppError::validation("Invalid profile path"))?;
        let stage = Self {
            root: root.to_path_buf(),
            workspace: parent.join(format!(".bepinex-install-{}", uuid::Uuid::new_v4())),
            keep_workspace: false,
        };
        fs::create_dir_all(stage.contents())?;
        Ok(stage)
    }

    /// Where the package is extracted before installation.
    pub fn contents(&self) -> PathBuf {
        self.workspace.join("contents")
    }

    fn backup(&self) -> PathBuf {
        self.workspace.join("backup")
    }

    pub fn install(&mut self, architecture: BinaryArch) -> AppResult<()> {
        BepInExRuntime::new(&self.contents()).validate(architecture)?;
        let mut units = Vec::new();
        self.collect_units(Path::new(""), &mut units)?;
        for (index, unit) in units.iter().enumerate() {
            if let Err(error) = self.swap(unit) {
                self.restore(&units[..=index]);
                return Err(error);
            }
        }
        Ok(())
    }

    /// Every package path that is replaced as a whole: files, plus the
    /// directories in `REPLACED_DIRS`. Everything else is merged.
    fn collect_units(&self, relative: &Path, units: &mut Vec<PathBuf>) -> AppResult<()> {
        let source = self.contents().join(relative);
        if source.is_dir() && !REPLACED_DIRS.iter().any(|dir| relative == Path::new(dir)) {
            let mut names = fs::read_dir(source)?
                .map(|entry| entry.map(|entry| entry.file_name()))
                .collect::<std::io::Result<Vec<_>>>()?;
            names.sort();
            for name in names {
                self.collect_units(&relative.join(name), units)?;
            }
        } else if !(PRESERVED_DIRS.iter().any(|dir| relative.starts_with(dir))
            && self.root.join(relative).exists())
        {
            units.push(relative.to_path_buf());
        }
        Ok(())
    }

    fn swap(&self, unit: &Path) -> AppResult<()> {
        let destination = self.root.join(unit);
        if fs::symlink_metadata(&destination).is_ok() {
            let backup = self.backup().join(unit);
            fs::create_dir_all(backup.parent().expect("unit is a relative path"))?;
            fs::rename(&destination, backup)?;
        }
        fs::create_dir_all(destination.parent().expect("unit is a relative path"))?;
        fs::rename(self.contents().join(unit), destination)?;
        Ok(())
    }

    /// Undo `swap` for `units`, newest first. A unit may be half swapped.
    fn restore(&mut self, units: &[PathBuf]) {
        for unit in units.iter().rev() {
            let destination = self.root.join(unit);
            let backup = self.backup().join(unit);
            let result = remove_path(&destination).and_then(|()| {
                if fs::symlink_metadata(&backup).is_ok() {
                    fs::rename(&backup, &destination)?;
                }
                Ok(())
            });
            if let Err(error) = result {
                self.keep_workspace = true;
                log::error!(
                    "Failed to restore {} from {}: {error}",
                    destination.display(),
                    backup.display()
                );
            }
        }
    }
}

fn remove_path(path: &Path) -> std::io::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() => fs::remove_dir_all(path),
        Ok(_) => fs::remove_file(path),
        // Nothing reachable there, e.g. a parent that is a file.
        Err(_) => Ok(()),
    }
}

impl Drop for StagedRuntime {
    fn drop(&mut self) {
        if self.keep_workspace {
            return;
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::test_support::{TempDir, write_test_runtime};

    #[test]
    fn failed_promotion_restores_the_previous_runtime() {
        let dir = TempDir::new("failed-promotion");
        let root = dir.0.join("profile");
        write_test_runtime(&root, 0x014c);
        let runtime = BepInExRuntime::new(&root);
        let original = fs::read(runtime.coreclr_path()).unwrap();

        let mut stage = StagedRuntime::new(&root).unwrap();
        write_test_runtime(&stage.contents(), 0x8664);
        // A file where the package needs a directory makes promotion fail
        // after earlier units have already been swapped.
        fs::write(stage.contents().join("a-file"), b"new").unwrap();
        fs::create_dir_all(stage.contents().join("z-dir")).unwrap();
        fs::write(stage.contents().join("z-dir/file"), b"new").unwrap();
        fs::write(root.join("z-dir"), b"blocks the directory").unwrap();

        assert!(stage.install(BinaryArch::X64).is_err());
        drop(stage);
        assert_eq!(runtime.installed_arch(), Some(BinaryArch::X86));
        assert_eq!(fs::read(runtime.coreclr_path()).unwrap(), original);
        assert!(!root.join("a-file").exists());
        assert_eq!(
            fs::read(root.join("z-dir")).unwrap(),
            b"blocks the directory"
        );
    }
}
