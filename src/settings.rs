//! Global mirror of the on-disk `AppSettings`.
//!
//! gpui-component's `Settings` widget wants `Fn(&App) -> T` value
//! readers and `Fn(T, &mut App)` setters. Routing those through a
//! `Context<SettingsView>` is awkward, so we keep the settings in a
//! gpui `Global` and have closures read/write the global directly. The
//! setter helper [`update`] also persists to disk through
//! `core_service::update_settings`.

use gpui::{App, Global};
use log::warn;

use crate::backend::services::core_service::{self, AppSettings, AppSettingsPatch};

pub struct SettingsGlobal(pub AppSettings);

impl Global for SettingsGlobal {}

pub fn init(cx: &mut App) {
    let initial = core_service::get_settings().unwrap_or_default();
    cx.set_global(SettingsGlobal(initial));
}

pub fn get(cx: &App) -> &AppSettings {
    &cx.global::<SettingsGlobal>().0
}

/// Apply `patch`, persist to disk, then write the result back to the
/// global. Errors are logged but not surfaced to the caller — settings
/// fields don't have a great way to report them inline.
pub fn update(cx: &mut App, patch: AppSettingsPatch) {
    match core_service::update_settings(patch) {
        Ok(new_settings) => {
            cx.set_global(SettingsGlobal(new_settings));
        }
        Err(e) => {
            warn!("update_settings failed: {e}");
        }
    }
}
