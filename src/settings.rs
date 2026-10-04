//! The on-disk `AppSettings`, mirrored in a global so the Settings widget's
//! `Fn(&App)` readers and setters can reach it.

use gpui_kit::{App, Global, Task};
use log::warn;

use crate::backend::error::AppResult;
use crate::backend::services::core_service::{self, AppSettings, GamePlatform};
use crate::backend::services::finder_service;

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

/// A detected Among Us install: its path and, when recognizable, its store.
pub type DetectedGame = (String, Option<GamePlatform>);

/// Find the Among Us install on the background executor and save it to the
/// settings. Shared by the first-run check and the Settings button, which
/// differ only in how they report it.
pub fn detect_among_us(cx: &mut App) -> Task<AppResult<Option<DetectedGame>>> {
    cx.spawn(async move |cx| {
        let found = cx
            .background_executor()
            .spawn(async {
                let path = finder_service::detect_among_us_installation()?;
                AppResult::Ok(path.map(|path| {
                    let store = finder_service::detect_game_store(&path).ok();
                    (path, store)
                }))
            })
            .await?;
        if let Some((path, store)) = &found {
            cx.update(|cx| {
                update(cx, |s| {
                    s.game.among_us_path = path.clone();
                    if let Some(platform) = *store {
                        s.game.game_platform = platform;
                    }
                })
            });
        }
        Ok(found)
    })
}
