//! Settings controls for linking and unlinking game installations.
use super::*;

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
                                    .child(div().flex_1().child(install.label()))
                                    .child(
                                        Button::new(SharedString::from(format!("unlink-{id}")))
                                            .label(t!("settings.unlink_install"))
                                            .on_click(move |_, _, cx| {
                                                app_settings::update(cx, |s| {
                                                    s.game_installations.retain(|i| i.id != id)
                                                });
                                            }),
                                    )
                            }),
                    )
                }),
            ),
        ])
}
