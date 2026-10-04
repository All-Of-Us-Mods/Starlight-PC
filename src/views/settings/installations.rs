//! Settings controls for linking and unlinking game installations.
use super::*;
use crate::backend::services::profile_service;
use gpui_kit::component::button::ButtonVariant;
use gpui_kit::component::dialog::DialogButtonProps;

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
                                let label = install.setup.label();
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(div().flex_1().child(label.clone()))
                                    .child(
                                        Button::new(SharedString::from(format!("unlink-{id}")))
                                            .label(t!("settings.unlink_install"))
                                            .on_click(move |_, window, cx| {
                                                confirm_unlink(
                                                    id.clone(),
                                                    label.clone(),
                                                    window,
                                                    cx,
                                                )
                                            }),
                                    )
                            }),
                    )
                }),
            ),
        ])
}

/// Ask before unlinking: its saved game and launch settings go with it.
fn confirm_unlink(id: String, label: String, window: &mut Window, cx: &mut App) {
    window.open_alert_dialog(cx, move |alert, _window, cx| {
        let id = id.clone();
        alert
            .icon(Icon::new(IconName::TriangleAlert).text_color(cx.theme().danger))
            .title(t!("settings.unlink_title"))
            .description(t!("settings.unlink_desc", name = label).to_string())
            .button_props(
                DialogButtonProps::default()
                    .ok_variant(ButtonVariant::Danger)
                    .ok_text(t!("settings.unlink_install"))
                    .cancel_text(t!("common.cancel"))
                    .show_cancel(true),
            )
            .on_ok(move |_, window, cx| {
                unlink(id.clone(), window, cx);
                true
            })
    });
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
