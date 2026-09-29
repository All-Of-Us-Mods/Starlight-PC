//! Theming on top of gpui-component's JSON theme system.
//!
//! Themes are plain `ThemeSet` JSON files (the same format gpui-component
//! ships in its `themes/` folder) living in `{app_data}/themes`. The bundled
//! Starlight palettes are written there on startup, the directory is watched
//! so edits apply live, and the active theme is remembered by name in
//! settings.

use std::path::{Path, PathBuf};
use std::rc::Rc;

use gpui_kit::component::{
    Theme as ComponentTheme, ThemeConfig, ThemeRegistry, scroll::ScrollbarMode,
};
use gpui_kit::*;
use log::warn;

use crate::backend::services::core_service::ScrollbarVisibility;

/// Starlight's own themes, written into the themes directory on startup.
const BUNDLED_THEMES: &str = include_str!("../assets/themes/starlight.json");
const BUNDLED_FILE_NAME: &str = "starlight.json";

/// Theme applied when settings name a theme that isn't installed.
pub const DEFAULT_THEME_NAME: &str = "Starlight Zinc";

/// Where user theme files live. Users can drop any gpui-component theme JSON
/// here; it shows up in the theme picker without a restart.
pub fn themes_dir() -> PathBuf {
    crate::backend::directories::app_data_dir()
        .unwrap_or_default()
        .join("themes")
}

/// Write the bundled themes into `dir`, overwriting the previous copy so app
/// updates ship palette fixes. Skipped when the content already matches, so
/// startup doesn't wake the directory watcher for nothing.
fn install_bundled_themes(dir: &Path) {
    if let Err(e) = std::fs::create_dir_all(dir) {
        warn!("failed to create themes dir {}: {e}", dir.display());
        return;
    }
    let path = dir.join(BUNDLED_FILE_NAME);
    if std::fs::read_to_string(&path).is_ok_and(|existing| existing == BUNDLED_THEMES) {
        return;
    }
    if let Err(e) = std::fs::write(&path, BUNDLED_THEMES) {
        warn!("failed to write {}: {e}", path.display());
    }
}

pub fn init(cx: &mut App) {
    let dir = themes_dir();
    install_bundled_themes(&dir);

    // Register the bundled themes synchronously so the first frame is already
    // themed — `watch_dir` loads the directory on a background task and would
    // otherwise leave a flash of gpui-component's default palette.
    if let Err(e) = ThemeRegistry::global_mut(cx).load_themes_from_str(BUNDLED_THEMES) {
        warn!("failed to load bundled themes: {e}");
    }
    apply_saved(cx);
    apply_scrollbar_visibility(cx);

    // Every registry change (initial directory load, or a file edited while
    // the app runs) re-applies the selected theme.
    cx.observe_global::<ThemeRegistry>(apply_saved).detach();

    if let Err(e) = ThemeRegistry::watch_dir(dir, cx, |_| {}) {
        warn!("failed to watch themes dir: {e}");
    }
}

/// Names of every installed theme, in the registry's display order.
pub fn theme_names(cx: &App) -> Vec<SharedString> {
    ThemeRegistry::global(cx)
        .sorted_themes()
        .iter()
        .map(|config| config.name.clone())
        .collect()
}

/// Re-apply the theme named in settings. Used on startup, after the theme
/// picker changes, and whenever the themes directory is reloaded.
pub fn apply_saved(cx: &mut App) {
    let name = crate::settings::get(cx).theme_name.clone();
    apply(cx, &name);
}

/// Install `name` as the active theme, falling back to the Starlight default
/// (and then to whatever the registry considers its default dark theme) when
/// it isn't installed.
pub fn apply(cx: &mut App, name: &str) {
    let registry = ThemeRegistry::global(cx);
    let config = registry
        .themes()
        .get(&SharedString::from(name.trim().to_string()))
        .or_else(|| {
            registry
                .themes()
                .get(&SharedString::new_static(DEFAULT_THEME_NAME))
        })
        .cloned()
        .unwrap_or_else(|| registry.default_dark_theme().clone());

    ComponentTheme::global_mut(cx).apply_config(&config);
    finish_apply(cx, &config);
}

/// Post-process a freshly applied theme: keep the window chrome transparent
/// unless the theme asked for its own (the workspace paints the background
/// and the starfield behind the sidebar and title bar), then repaint.
fn finish_apply(cx: &mut App, config: &Rc<ThemeConfig>) {
    let transparent = gpui_kit::transparent_black();
    let theme = ComponentTheme::global_mut(cx);

    if config.colors.sidebar.is_none() {
        theme.colors.sidebar = transparent;
        theme.tokens.sidebar = transparent.into();
    }
    if config.colors.title_bar.is_none() {
        theme.colors.title_bar = transparent;
        theme.tokens.title_bar = transparent.into();
    }

    // Scrollbars paint from gpui-component's Base layer, which keeps its own copy
    // of the theme; `apply_config` alone leaves it on the previous palette.
    ComponentTheme::sync_base(cx);
    cx.refresh_windows();
}

/// Push the saved scrollbar visibility down to gpui-component, which owns the
/// overlay scrollbars every scrolling view renders.
pub fn apply_scrollbar_visibility(cx: &mut App) {
    let mode = match crate::settings::get(cx).scrollbar_visibility {
        ScrollbarVisibility::Scrolling => ScrollbarMode::Scrolling,
        ScrollbarVisibility::Hover => ScrollbarMode::Hover,
        ScrollbarVisibility::Always => ScrollbarMode::Always,
    };
    ComponentTheme::set_scrollbar_mode(mode, cx);
}

#[cfg(test)]
mod tests {
    // Deliberately not `use super::*`: that pulls in `gpui_kit::*`, whose `test`
    // attribute macro would shadow the built-in `#[test]`.
    use super::{BUNDLED_THEMES, DEFAULT_THEME_NAME};

    /// Deserializing as `serde_json::Value` rather than `ThemeSet` on purpose:
    /// instantiating the generated `ThemeSet` deserializer here overflows
    /// rustc's stack. The registry parses the real thing at runtime; this
    /// guards the file's shape.
    #[test]
    fn bundled_themes_parse_and_include_the_default() {
        let set: serde_json::Value =
            serde_json::from_str(BUNDLED_THEMES).expect("bundled themes are valid JSON");
        let themes = set["themes"].as_array().expect("themes array");

        assert!(
            themes.iter().any(|t| t["name"] == DEFAULT_THEME_NAME),
            "bundled set must contain the fallback theme"
        );
        for theme in themes {
            assert_eq!(theme["mode"], "dark", "{}", theme["name"]);
            for key in ["background", "foreground", "primary.background"] {
                assert!(
                    theme["colors"][key].is_string(),
                    "{} is missing {key}",
                    theme["name"]
                );
            }
        }
    }
}
