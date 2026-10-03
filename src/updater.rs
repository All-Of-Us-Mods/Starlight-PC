//! The self-updater's app-owned state: one [`UpdateState`] shared by the
//! startup check, Settings → Updates, and the "update available"
//! notification. Every transition goes through it, which is what makes a
//! second check or install while one is running a no-op.
//!
//! Only Windows can install an update in place (see `update_service`), so
//! the callers only reach this on Windows; it compiles everywhere so it can
//! be built and tested off Windows too.

use gpui_kit::component::WindowExt as _;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::notification::Notification;
use gpui_kit::*;
use log::warn;
use rust_i18n::t;

use crate::backend::error::AppResult;
use crate::backend::services::update_service::{self, UpdateInfo};
use crate::settings as app_settings;

#[derive(Debug, Clone, Default)]
pub enum UpdateState {
    #[default]
    Idle,
    Checking,
    UpToDate,
    Available(UpdateInfo),
    /// `percent` stays `None` while the server hasn't said how big the
    /// download is.
    Downloading {
        info: UpdateInfo,
        percent: Option<u8>,
    },
    /// `update` is what a retry installs: the release that failed to
    /// install, or `None` if the check itself failed.
    Failed {
        error: String,
        update: Option<UpdateInfo>,
    },
}

impl UpdateState {
    pub fn is_busy(&self) -> bool {
        matches!(self, Self::Checking | Self::Downloading { .. })
    }

    // Each transition returns whether it applied, so the caller only
    // republishes (and redraws) on a real change.

    fn begin_check(&mut self) -> bool {
        if self.is_busy() {
            return false;
        }
        *self = Self::Checking;
        true
    }

    fn finish_check(&mut self, result: Result<Option<UpdateInfo>, String>) -> bool {
        if !matches!(self, Self::Checking) {
            return false;
        }
        *self = match result {
            Ok(Some(info)) => Self::Available(info),
            Ok(None) => Self::UpToDate,
            Err(error) => Self::Failed {
                error,
                update: None,
            },
        };
        true
    }

    fn begin_install(&mut self, info: UpdateInfo) -> bool {
        if self.is_busy() {
            return false;
        }
        *self = Self::Downloading {
            info,
            percent: None,
        };
        true
    }

    fn set_progress(&mut self, new: Option<u8>) -> bool {
        match self {
            Self::Downloading { percent, .. } if *percent != new => {
                *percent = new;
                true
            }
            _ => false,
        }
    }

    fn fail_install(&mut self, error: String) -> bool {
        let Self::Downloading { info, .. } = self else {
            return false;
        };
        *self = Self::Failed {
            error,
            update: Some(info.clone()),
        };
        true
    }
}

/// Whole download percentage, or `None` without a known size. The download
/// reports every chunk; only a change in this is worth a redraw.
fn percent(downloaded: u64, total: Option<u64>) -> Option<u8> {
    let total = total.filter(|&total| total > 0)?;
    Some((downloaded.min(total) * 100 / total) as u8)
}

pub struct UpdateGlobal(UpdateState);

impl Global for UpdateGlobal {}

pub fn init(cx: &mut App) {
    cx.set_global(UpdateGlobal(UpdateState::default()));
}

pub fn state(cx: &App) -> &UpdateState {
    &cx.global::<UpdateGlobal>().0
}

/// Apply `transition` and republish the state if it changed.
fn transition(cx: &mut App, transition: impl FnOnce(&mut UpdateState) -> bool) -> bool {
    let mut state = state(cx).clone();
    let changed = transition(&mut state);
    if changed {
        cx.set_global(UpdateGlobal(state));
    }
    changed
}

/// Check the user's release channel for a build newer than this one and, if
/// there is one, offer it in a notification. Ignored while busy.
pub fn check(window: &mut Window, cx: &mut App) {
    if !transition(cx, UpdateState::begin_check) {
        return;
    }
    let channel = app_settings::get(cx).release_channel;
    let window_handle = window.window_handle();
    cx.spawn(async move |cx| {
        let result = cx
            .background_executor()
            .spawn(async move { update_service::check_for_update(channel) })
            .await
            .map_err(|e| {
                warn!("update check failed: {e}");
                e.to_string()
            });
        let found = result.clone().ok().flatten();
        cx.update(|cx| transition(cx, |s| s.finish_check(result)));
        if let Some(info) = found {
            let _ = window_handle.update(cx, |_, window, cx| {
                window.push_notification(notification(info), cx);
            });
        }
    })
    .detach();
}

/// Download `info`, swap it in, relaunch, and quit. Ignored while busy.
pub fn install(info: UpdateInfo, window: &mut Window, cx: &mut App) {
    if !transition(cx, |s| s.begin_install(info.clone())) {
        return;
    }
    let window_handle = window.window_handle();
    cx.spawn(async move |cx| {
        // Only whole-percent changes are sent, so 0..=100 always fits.
        let (tx, mut rx) = async_broadcast::broadcast::<Option<u8>>(101);
        let work = cx.background_executor().spawn(async move {
            let mut last = None;
            apply(&info, move |downloaded, total| {
                let now = percent(downloaded, total);
                if now != last {
                    last = now;
                    let _ = tx.try_broadcast(now);
                }
            })
        });
        // Ends when the download drops the sender.
        while let Ok(percent) = rx.recv().await {
            cx.update(|cx| transition(cx, |s| s.set_progress(percent)));
        }
        match work.await {
            Ok(()) => cx.update(|cx| cx.quit()),
            Err(e) => {
                warn!("update install failed: {e}");
                cx.update(|cx| transition(cx, |s| s.fail_install(e.to_string())));
                let _ = window_handle.update(cx, |_, window, cx| {
                    window.push_notification(
                        Notification::error(t!("update.failed", error = e).to_string()),
                        cx,
                    );
                });
            }
        }
    })
    .detach();
}

#[cfg(windows)]
fn apply(info: &UpdateInfo, on_progress: impl FnMut(u64, Option<u64>)) -> AppResult<()> {
    update_service::apply_update_and_relaunch(info, on_progress)
}

#[cfg(not(windows))]
fn apply(_: &UpdateInfo, _: impl FnMut(u64, Option<u64>)) -> AppResult<()> {
    Err(crate::backend::error::AppError::validation(
        "Self-update is only supported on Windows",
    ))
}

/// The button that installs `info`; inert while a check or install runs.
pub fn install_button(info: UpdateInfo, cx: &App) -> Button {
    Button::new("install-update")
        .label(t!("update.restart"))
        .primary()
        .loading(state(cx).is_busy())
        .on_click(move |_, window, cx| install(info.clone(), window, cx))
}

fn notification(info: UpdateInfo) -> Notification {
    Notification::info(t!("update.available", version = info.version).to_string())
        .id::<UpdateGlobal>()
        .title(t!("update.title"))
        .action(move |_, _, cx| install_button(info.clone(), cx))
}

#[cfg(test)]
mod tests {
    // Not `super::*`: that brings in `gpui_kit::*`, whose `test` attribute
    // shadows std's.
    use super::{UpdateInfo, UpdateState, percent};

    fn info(version: &str) -> UpdateInfo {
        UpdateInfo {
            version: version.into(),
            download_url: String::new(),
            expected_sha256: None,
        }
    }

    fn downloading(state: &UpdateState) -> Option<(&str, Option<u8>)> {
        match state {
            UpdateState::Downloading { info, percent } => Some((&info.version, *percent)),
            _ => None,
        }
    }

    #[test]
    fn check_finds_an_update_or_reports_up_to_date() {
        let mut state = UpdateState::default();
        assert!(state.begin_check());
        assert!(state.finish_check(Ok(Some(info("2.0.0")))));
        assert!(matches!(&state, UpdateState::Available(i) if i.version == "2.0.0"));

        assert!(state.begin_check());
        assert!(state.finish_check(Ok(None)));
        assert!(matches!(state, UpdateState::UpToDate));
    }

    #[test]
    fn second_check_while_checking_is_ignored() {
        let mut state = UpdateState::default();
        assert!(state.begin_check());
        assert!(!state.begin_check());
        assert!(!state.begin_install(info("2.0.0")));
        assert!(matches!(state, UpdateState::Checking));
    }

    #[test]
    fn second_install_while_downloading_is_ignored() {
        let mut state = UpdateState::Available(info("2.0.0"));
        assert!(state.begin_install(info("2.0.0")));
        assert!(state.set_progress(Some(40)));
        assert!(!state.begin_install(info("2.0.1")));
        assert!(!state.begin_check());
        assert_eq!(downloading(&state), Some(("2.0.0", Some(40))));
    }

    #[test]
    fn failed_install_keeps_the_release_for_retry() {
        let mut state = UpdateState::default();
        assert!(state.begin_install(info("2.0.0")));
        assert!(state.fail_install("checksum mismatch".into()));
        let UpdateState::Failed { error, update } = &state else {
            panic!("expected Failed, got {state:?}");
        };
        assert_eq!(error, "checksum mismatch");
        assert_eq!(update.as_ref().map(|i| i.version.as_str()), Some("2.0.0"));
        // A retry is allowed once the failure has landed.
        assert!(state.begin_install(info("2.0.0")));
    }

    #[test]
    fn failed_check_has_nothing_to_retry_but_can_check_again() {
        let mut state = UpdateState::default();
        assert!(state.begin_check());
        assert!(state.finish_check(Err("offline".into())));
        assert!(matches!(state, UpdateState::Failed { update: None, .. }));
        assert!(state.begin_check());
    }

    #[test]
    fn progress_only_changes_on_a_new_percentage_while_downloading() {
        let mut state = UpdateState::default();
        assert!(!state.set_progress(Some(10)), "not downloading");
        assert!(state.begin_install(info("2.0.0")));
        assert!(state.set_progress(Some(10)));
        assert!(!state.set_progress(Some(10)));
        assert!(state.set_progress(Some(11)));
    }

    #[test]
    fn percent_is_whole_and_only_known_with_a_size() {
        assert_eq!(percent(0, Some(1000)), Some(0));
        assert_eq!(percent(9, Some(1000)), Some(0));
        assert_eq!(percent(10, Some(1000)), Some(1));
        assert_eq!(percent(999, Some(1000)), Some(99));
        assert_eq!(percent(1000, Some(1000)), Some(100));
        assert_eq!(percent(2000, Some(1000)), Some(100));
        assert_eq!(percent(500, None), None);
        assert_eq!(percent(500, Some(0)), None);
    }

    #[test]
    fn a_large_download_sends_at_most_one_update_per_percent() {
        // Mirrors the throttle in `install`: 64 KiB chunks of a 50 MB file.
        let total = 50_000_000;
        let mut last = None;
        let mut sent = 0;
        for downloaded in (0..=total).step_by(64 * 1024).chain([total]) {
            let now = percent(downloaded, Some(total));
            if now != last {
                last = now;
                sent += 1;
            }
        }
        assert_eq!(sent, 101);
    }
}
