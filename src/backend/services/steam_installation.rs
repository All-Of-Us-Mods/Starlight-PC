//! Verify the launch target before Steam's app-id launcher changes any files.

use super::installation_service::GAME_EXE_NAME;
use crate::backend::error::{AppError, AppResult};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

const APP_ID: &str = "945360";

pub(super) fn validate_directory(selected: &Path, steam_roots: &[PathBuf]) -> AppResult<()> {
    let selected = selected.canonicalize()?;
    match directories(steam_roots)?.as_slice() {
        [game_dir] if game_dir == &selected => Ok(()),
        [game_dir] => Err(AppError::validation(format!(
            "Steam launches Among Us from {}. Select that installation, or use Wine or Proton to launch {} directly.",
            game_dir.display(),
            selected.display()
        ))),
        _ => Err(AppError::validation(
            "Could not uniquely identify Steam's registered Among Us installation. Use Wine or Proton to launch the selected folder directly.",
        )),
    }
}

/// Shared by auto-detection and launch validation, including external libraries
/// and custom installation folder names recorded in Steam's manifests.
pub(super) fn directories(steam_roots: &[PathBuf]) -> AppResult<Vec<PathBuf>> {
    let mut libraries: BTreeSet<_> = steam_roots.iter().cloned().collect();
    for root in steam_roots {
        if let Some(raw) = read_optional(&root.join("steamapps/libraryfolders.vdf"))? {
            libraries.extend(quoted_values(&raw, "path").map(PathBuf::from));
        }
    }
    let mut registered = BTreeSet::new();
    for library in libraries {
        let path = library.join(format!("steamapps/appmanifest_{APP_ID}.acf"));
        let Some(raw) = read_optional(&path)? else {
            continue;
        };
        let folder = quoted_values(&raw, "installdir")
            .next()
            .filter(|folder| {
                Path::new(folder)
                    .file_name()
                    .is_some_and(|name| name == folder.as_str())
            })
            .filter(|_| quoted_values(&raw, "appid").next().as_deref() == Some(APP_ID))
            .ok_or_else(|| {
                AppError::validation(format!("Invalid Steam manifest at {}.", path.display()))
            })?;
        let game_dir = library.join("steamapps/common").join(folder);
        if game_dir.join(GAME_EXE_NAME).is_file() {
            registered.insert(game_dir.canonicalize()?);
        }
    }
    Ok(registered.into_iter().collect())
}

fn read_optional(path: &Path) -> std::io::Result<Option<String>> {
    match fs::read_to_string(path) {
        Ok(raw) => Ok(Some(raw)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

// Steam's library and app manifests store these fields as quoted key/value
// pairs. Keep paths with spaces and unescape the doubled backslashes in VDF.
fn quoted_values<'a>(raw: &'a str, key: &'a str) -> impl Iterator<Item = String> + 'a {
    raw.lines().filter_map(move |line| {
        let mut parts = line.trim().split('"');
        if !parts.next()?.is_empty() || parts.next()? != key {
            return None;
        }
        parts.next()?;
        Some(parts.next()?.replace("\\\\", "\\"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::test_support::TempDir;

    #[test]
    fn validates_registered_library_and_rejects_an_unregistered_copy() {
        let dir = TempDir::new("steam-target");
        let root = dir.0.join("client");
        let library = dir.0.join("external library");
        let registered = library.join("steamapps/common/Among Us");
        let copy = dir.0.join("linked copy");
        fs::create_dir_all(root.join("steamapps")).unwrap();
        fs::create_dir_all(&registered).unwrap();
        fs::create_dir_all(&copy).unwrap();
        fs::write(registered.join(GAME_EXE_NAME), b"game").unwrap();
        fs::write(copy.join(GAME_EXE_NAME), b"game").unwrap();
        fs::write(
            root.join("steamapps/libraryfolders.vdf"),
            format!(
                "\"libraryfolders\"\n{{\n \"0\"\n {{\n  \"path\" \"{}\"\n }}\n}}",
                library.to_string_lossy().replace('\\', "\\\\")
            ),
        )
        .unwrap();
        fs::write(
            library.join("steamapps/appmanifest_945360.acf"),
            "\"AppState\"\n{\n \"appid\" \"945360\"\n \"installdir\" \"Among Us\"\n}",
        )
        .unwrap();
        assert!(validate_directory(&registered, std::slice::from_ref(&root)).is_ok());
        assert!(validate_directory(&copy, &[root]).is_err());
        assert!(!copy.join("winhttp.dll").exists());
        assert!(!copy.join("doorstop_config.ini").exists());
    }

    #[test]
    fn refuses_missing_or_ambiguous_steam_registration() {
        let dir = TempDir::new("ambiguous-steam-target");
        assert!(validate_directory(&dir.0, &[]).is_err());
        let roots: Vec<PathBuf> = ["first", "second"]
            .iter()
            .map(|name| dir.0.join(name))
            .collect();
        for root in &roots {
            let game = root.join("steamapps/common/Among Us");
            fs::create_dir_all(&game).unwrap();
            fs::write(game.join(GAME_EXE_NAME), b"game").unwrap();
            fs::write(
                root.join("steamapps/appmanifest_945360.acf"),
                "\"appid\" \"945360\"\n\"installdir\" \"Among Us\"",
            )
            .unwrap();
        }
        assert!(validate_directory(&roots[0].join("steamapps/common/Among Us"), &roots).is_err());
    }
}
