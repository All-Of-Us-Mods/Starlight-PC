//! Settings controls for linking and unlinking game installations.
use super::*;
use crate::backend::services::profile_service;

pub(super) fn group() -> SettingGroup {
    SettingGroup::new()
        .title(t!("settings.linked_installs"))
        .items(vec![
            stacked_item(
                t!("settings.link_install"),
                SettingField::render(|_, _, _| {
                    Button::new("link-install")
                        .label(t!("settings.link_install"))
                        .on_click(|_, window, cx| {
                            let mut settings = app_settings::get(cx).clone();
                            match settings.link_current_installation() {
                                Ok(()) => app_settings::update(cx, |s| {
                                    s.game_installations = settings.game_installations
                                }),
                                Err(e) => {
                                    window.push_notification(Notification::error(e.to_string()), cx)
                                }
                            }
                        })
                }),
            )
            .description(t!("settings.link_install_desc").to_string()),
            stacked_item(
                t!("settings.linked_installs"),
                SettingField::render(|_, _, cx| {
                    div().flex().flex_col().gap_2().children(
                        app_settings::get(cx)
                            .game_installations
                            .clone()
                            .into_iter()
                            .map(|install| {
                                let id = install.id.clone();
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(div().flex_1().child(install.setup.label()))
                                    .child(
                                        Button::new(SharedString::from(format!("unlink-{id}")))
                                            .label(t!("settings.unlink_install"))
                                            .on_click(move |_, window, cx| {
                                                unlink(id.clone(), window, cx)
                                            }),
                                    )
                            }),
                    )
                }),
            ),
        ])
}

/// Unlink installation `id` unless a profile still launches through it. The
/// profile scan runs on the background executor.
fn unlink(id: String, window: &mut Window, cx: &mut App) {
    let profiles = cx
        .background_executor()
        .spawn(async { profile_service::get_profiles() });
    window
        .spawn(cx, async move |cx| {
            let profiles = profiles.await;
            let _ = cx.update(|window, cx| {
                let mut settings = app_settings::get(cx).clone();
                match profiles.and_then(|profiles| settings.unlink_installation(&id, &profiles)) {
                    Ok(()) => app_settings::update(cx, |s| {
                        s.game_installations = settings.game_installations
                    }),
                    Err(error) => {
                        window.push_notification(Notification::error(error.to_string()), cx)
                    }
                }
            });
        })
        .detach();
}
