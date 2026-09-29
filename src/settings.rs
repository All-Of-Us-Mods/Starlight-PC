//! The on-disk `AppSettings`, mirrored in a global so the Settings widget's
//! `Fn(&App)` readers and setters can reach it.

use gpui_kit::{App, Global};
use log::warn;

use crate::backend::services::core_service::{self, AppSettings};

pub struct SettingsGlobal(pub AppSettings);

impl Global for SettingsGlobal {}

pub fn init(cx: &mut App) {
    let initial = core_service::get_settings().unwrap_or_default();
    cx.set_global(SettingsGlobal(initial));
}

pub fn get(cx: &App) -> &AppSettings {
    &cx.global::<SettingsGlobal>().0
}

/// Edit, persist, and republish the settings. Failures are only logged.
pub fn update(cx: &mut App, edit: impl FnOnce(&mut AppSettings)) {
    match core_service::update_settings(edit) {
        Ok(settings) => cx.set_global(SettingsGlobal(settings)),
        Err(e) => warn!("update_settings failed: {e}"),
    }
}
