//! Desktop shortcuts that launch a profile via the `starlight://` deep link.
//!
//! Windows-only: the shortcut is an `[InternetShortcut]` `.url` file written
//! to the desktop, and the scheme is registered per-user under
//! `HKCU\Software\Classes` so no elevation is needed. Opening the shortcut
//! makes the shell start the app with the URL as its first argument, which
//! `main` picks up via [`parse_profile_deep_link`] to auto-launch the profile.

pub const DEEP_LINK_SCHEME: &str = "starlight";
pub const PROFILE_LINK_HOST: &str = "profile";

/// Extract the profile id from a `starlight://profile/{id}` deep link, as
/// passed in argv when the shell opens the registered scheme.
pub fn parse_profile_deep_link(arg: &str) -> Option<String> {
    let prefix = format!("{DEEP_LINK_SCHEME}://{PROFILE_LINK_HOST}/");
    let rest = arg.strip_prefix(&prefix)?;
    let id = rest.trim_end_matches('/');
    if id.is_empty() || id.contains('/') {
        return None;
    }
    let id = urlencoding::decode(id).ok()?.trim().to_string();
    (!id.is_empty()).then_some(id)
}

#[cfg(windows)]
mod windows_impl {
    use super::{DEEP_LINK_SCHEME, PROFILE_LINK_HOST};
    use crate::backend::error::{AppError, AppResult};
    use crate::backend::services::profile_service;
    use std::fs;

    const SHORTCUT_PREFIX: &str = "Starlight - ";

    /// Register (or refresh) the `starlight://` scheme for the current user so
    /// deep links open this executable. Cheap enough to run on every startup,
    /// which also keeps the registered path current when the app moves.
    pub fn register_deep_link_scheme() -> AppResult<()> {
        use winreg::RegKey;
        use winreg::enums::HKEY_CURRENT_USER;

        let exe = std::env::current_exe()?.to_string_lossy().to_string();
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let (key, _) = hkcu.create_subkey(format!(r"Software\Classes\{DEEP_LINK_SCHEME}"))?;
        key.set_value("", &format!("URL:{DEEP_LINK_SCHEME} Protocol"))?;
        key.set_value("URL Protocol", &"")?;
        let (icon, _) = key.create_subkey("DefaultIcon")?;
        icon.set_value("", &format!("\"{exe}\",0"))?;
        let (command, _) = key.create_subkey(r"shell\open\command")?;
        command.set_value("", &format!("\"{exe}\" \"%1\""))?;
        Ok(())
    }

    /// Write a `.url` shortcut for the profile onto the user's desktop.
    /// Returns the shortcut's path.
    pub fn create_desktop_shortcut(profile_id: &str) -> AppResult<String> {
        let Some(profile) = profile_service::get_profile_by_id(profile_id)? else {
            return Err(AppError::validation(format!(
                "Profile '{profile_id}' not found"
            )));
        };

        let desktop_dir = directories::UserDirs::new()
            .and_then(|dirs| dirs.desktop_dir().map(|p| p.to_path_buf()))
            .ok_or_else(|| AppError::platform("Could not determine the desktop directory"))?;
        fs::create_dir_all(&desktop_dir)?;

        let shortcut_name = sanitize_shortcut_name(&profile.name);
        let shortcut_path = desktop_dir.join(format!("{SHORTCUT_PREFIX}{shortcut_name}.url"));
        let shortcut_url = format!(
            "{DEEP_LINK_SCHEME}://{PROFILE_LINK_HOST}/{}",
            urlencoding::encode(&profile.id)
        );
        // .url icons must be .ico/.exe/.dll — point at the app executable.
        let exe = std::env::current_exe()?;
        let contents = format!(
            "[InternetShortcut]\r\nURL={shortcut_url}\r\nIconFile={}\r\nIconIndex=0\r\n",
            exe.display()
        );
        fs::write(&shortcut_path, contents)?;

        Ok(shortcut_path.to_string_lossy().to_string())
    }

    /// Strip characters Windows forbids in file names, so the profile name can
    /// be used as the shortcut file name.
    fn sanitize_shortcut_name(name: &str) -> String {
        let cleaned: String = name
            .chars()
            .map(|ch| match ch {
                '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '-',
                c if c.is_control() => '-',
                c => c,
            })
            .collect();
        let cleaned = cleaned.trim().trim_end_matches('.').to_string();
        if cleaned.is_empty() {
            "Profile".to_string()
        } else {
            cleaned
        }
    }
}

#[cfg(windows)]
pub use windows_impl::{create_desktop_shortcut, register_deep_link_scheme};

#[cfg(test)]
mod tests {
    use super::parse_profile_deep_link;

    #[test]
    fn parses_profile_deep_link() {
        assert_eq!(
            parse_profile_deep_link("starlight://profile/my-profile-123"),
            Some("my-profile-123".to_string())
        );
        // Shell-appended trailing slash.
        assert_eq!(
            parse_profile_deep_link("starlight://profile/my-profile-123/"),
            Some("my-profile-123".to_string())
        );
        assert_eq!(parse_profile_deep_link("starlight://profile/"), None);
        assert_eq!(parse_profile_deep_link("starlight://profile/a/b"), None);
        assert_eq!(parse_profile_deep_link("starlight://other/x"), None);
        assert_eq!(parse_profile_deep_link("--flag"), None);
    }
}
