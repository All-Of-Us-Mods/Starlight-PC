//! A profile's `BepInEx/plugins` folder. BepInEx loads every `*.dll` in it,
//! including subfolders and symlinks. Disabled plugins are renamed to
//! `<file>.disabled`.

use crate::backend::error::{AppError, AppResult};
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::io::ErrorKind;
use std::path::{Component, Path, PathBuf};

const DISABLED_SUFFIX: &str = ".disabled";

pub struct Plugins {
    dir: PathBuf,
}

impl Plugins {
    pub fn new(profile_dir: &Path) -> Self {
        Self {
            dir: profile_dir.join("BepInEx").join("plugins"),
        }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Plugin files, as `/`-separated paths relative to the folder, mapped to
    /// whether they're enabled.
    pub fn scan(&self) -> BTreeMap<String, bool> {
        let mut found = BTreeMap::new();
        scan_dir(&self.dir, "", &mut HashSet::new(), &mut found);
        found
    }

    pub fn set_enabled(&self, file: &str, enabled: bool) -> AppResult<()> {
        let [on, off] = self.paths(file)?;
        let (from, to) = if enabled { (off, on) } else { (on, off) };
        if to.exists() {
            return Ok(());
        }
        Ok(fs::rename(from, to)?)
    }

    pub fn remove(&self, file: &str) -> AppResult<()> {
        for path in self.paths(file)? {
            match fs::remove_file(path) {
                Err(error) if error.kind() != ErrorKind::NotFound => return Err(error.into()),
                _ => {}
            }
        }
        Ok(())
    }

    /// Copy a `.dll` into the folder, enabled. Returns its file name.
    pub fn import(&self, source: &Path) -> AppResult<String> {
        if !source.is_file() {
            return Err(AppError::validation("Selected mod file does not exist"));
        }
        let name = source
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| name.to_ascii_lowercase().ends_with(".dll"))
            .ok_or_else(|| AppError::validation("Selected file must be a .dll"))?;
        fs::create_dir_all(&self.dir)?;
        fs::copy(source, self.dir.join(name))?;
        let _ = fs::remove_file(self.dir.join(format!("{name}{DISABLED_SUFFIX}")));
        Ok(name.to_string())
    }

    /// The enabled and disabled paths of `file`. Plugins inside a symlinked
    /// folder are refused: renaming or deleting them would change files
    /// outside the profile.
    fn paths(&self, file: &str) -> AppResult<[PathBuf; 2]> {
        if !is_valid_file(file) {
            return Err(AppError::validation("Invalid mod file name"));
        }
        let mut dir = self.dir.clone();
        for component in Path::new(file)
            .parent()
            .into_iter()
            .flat_map(Path::components)
        {
            dir.push(component);
            if fs::symlink_metadata(&dir).is_ok_and(|metadata| metadata.is_symlink()) {
                return Err(AppError::validation(
                    "This plugin is in a linked folder; manage it from its source folder",
                ));
            }
        }
        Ok([
            self.dir.join(file),
            self.dir.join(format!("{file}{DISABLED_SUFFIX}")),
        ])
    }
}

/// A relative, `/`-separated path that stays inside the plugins folder.
pub fn is_valid_file(file: &str) -> bool {
    !file.is_empty()
        && !file.contains('\\')
        && Path::new(file)
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn scan_dir(
    dir: &Path,
    prefix: &str,
    visited: &mut HashSet<PathBuf>,
    found: &mut BTreeMap<String, bool>,
) {
    let Ok(canonical) = fs::canonicalize(dir) else {
        return;
    };
    if !visited.insert(canonical) {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let Some(name) = entry.file_name().to_str().map(str::to_string) else {
            continue;
        };
        let path = format!("{prefix}{name}");
        if entry.path().is_dir() {
            scan_dir(&entry.path(), &format!("{path}/"), visited, found);
            continue;
        }
        let lower = name.to_ascii_lowercase();
        if lower.ends_with(".dll") {
            found.insert(path, true);
        } else if lower.ends_with(".dll.disabled") {
            let file = path[..path.len() - DISABLED_SUFFIX.len()].to_string();
            found.entry(file).or_insert(false);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(tag: &str) -> Self {
            let dir = std::env::temp_dir()
                .join(format!("starlight-plugins-{tag}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(dir.join("BepInEx").join("plugins")).unwrap();
            Self(dir)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn scan_finds_enabled_and_disabled_plugins_in_subfolders() {
        let dir = TempDir::new("scan");
        let plugins = Plugins::new(&dir.0);
        let nested = plugins.dir().join("sinai-dev-UnityExplorer");
        fs::create_dir_all(&nested).unwrap();
        fs::write(plugins.dir().join("Loose.dll"), b"x").unwrap();
        fs::write(plugins.dir().join("notes.txt"), b"x").unwrap();
        fs::write(nested.join("UnityExplorer.dll"), b"x").unwrap();
        fs::write(nested.join("Off.dll.disabled"), b"x").unwrap();
        fs::write(nested.join("Both.dll"), b"x").unwrap();
        fs::write(nested.join("Both.dll.disabled"), b"x").unwrap();

        assert_eq!(
            plugins.scan(),
            BTreeMap::from([
                ("Loose.dll".to_string(), true),
                ("sinai-dev-UnityExplorer/Both.dll".to_string(), true),
                ("sinai-dev-UnityExplorer/Off.dll".to_string(), false),
                (
                    "sinai-dev-UnityExplorer/UnityExplorer.dll".to_string(),
                    true
                ),
            ])
        );
    }

    #[cfg(unix)]
    #[test]
    fn scan_follows_symlinks_once_and_only_touches_links() {
        use std::os::unix::fs::symlink;
        let dir = TempDir::new("symlinks");
        let plugins = Plugins::new(&dir.0);
        let outside = dir.0.join("outside");
        fs::create_dir_all(&outside).unwrap();
        fs::write(outside.join("Linked.dll"), b"x").unwrap();
        symlink(outside.join("Linked.dll"), plugins.dir().join("Linked.dll")).unwrap();
        symlink(&outside, plugins.dir().join("linked-dir")).unwrap();
        symlink(plugins.dir(), plugins.dir().join("loop")).unwrap();

        let files: Vec<_> = plugins.scan().into_keys().collect();
        assert_eq!(files, ["Linked.dll", "linked-dir/Linked.dll"]);

        assert!(plugins.remove("linked-dir/Linked.dll").is_err());
        assert!(plugins.set_enabled("linked-dir/Linked.dll", false).is_err());
        plugins.remove("Linked.dll").unwrap();
        assert!(outside.join("Linked.dll").is_file());
    }

    #[test]
    fn set_enabled_renames_and_remove_deletes_both_states() {
        let dir = TempDir::new("toggle");
        let plugins = Plugins::new(&dir.0);
        fs::write(plugins.dir().join("Mod.dll"), b"x").unwrap();

        plugins.set_enabled("Mod.dll", false).unwrap();
        assert!(!plugins.scan()["Mod.dll"]);
        plugins.set_enabled("Mod.dll", false).unwrap();
        plugins.set_enabled("Mod.dll", true).unwrap();
        assert!(plugins.scan()["Mod.dll"]);

        fs::write(plugins.dir().join("Mod.dll.disabled"), b"x").unwrap();
        plugins.remove("Mod.dll").unwrap();
        assert!(plugins.scan().is_empty());
        assert!(plugins.set_enabled("Mod.dll", true).is_err());
    }

    #[test]
    fn valid_files_stay_inside_the_folder() {
        assert!(is_valid_file("Mod.dll"));
        assert!(is_valid_file("folder/Mod.dll"));
        assert!(!is_valid_file("../Mod.dll"));
        assert!(!is_valid_file("/etc/Mod.dll"));
        assert!(!is_valid_file("folder\\Mod.dll"));
        assert!(!is_valid_file(""));
    }
}
