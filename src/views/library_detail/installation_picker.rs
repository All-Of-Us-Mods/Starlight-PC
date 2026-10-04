//! Persistent launch target selection for a profile.
use super::*;
use gpui_kit::component::menu::{DropdownMenu, PopupMenuItem};

impl LibraryDetailView {
    pub(super) fn installation_picker(
        &self,
        profile: &ProfileEntry,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let selected = profile.installation_id.clone();
        let installs = app_settings::get(cx).game_installations.clone();
        let label = match profile.installation(app_settings::get(cx)) {
            Ok(game) => game.game_platform.display_name().to_string(),
            Err(_) => t!("profile.missing_install").to_string(),
        };
        let view = cx.entity().downgrade();
        Button::new("profile-installation")
            .label(label)
            .disabled(
                self.bep_progress.is_some()
                    || !self.updating_mods.is_empty()
                    || self.running_count > 0,
            )
            .dropdown_menu(move |mut menu, _, _| {
                let options = std::iter::once((None, t!("profile.default_install").to_string()))
                    .chain(
                        installs
                            .iter()
                            .map(|i| (Some(i.id.clone()), i.setup.label())),
                    );
                for (id, label) in options {
                    let view = view.clone();
                    menu = menu.item(PopupMenuItem::new(label).checked(id == selected).on_click(
                        move |_, _, cx| {
                            let id = id.clone();
                            let _ = view.update(cx, |this, cx| this.set_installation(id, cx));
                        },
                    ));
                }
                menu
            })
    }

    fn set_installation(&mut self, installation_id: Option<String>, cx: &mut Context<Self>) {
        let profile_id = self.profile_id.clone();
        cx.spawn(async move |this, cx| {
            let id = installation_id.clone();
            let result = cx
                .background_executor()
                .spawn(async move { profile_service::set_installation(&profile_id, id) })
                .await;
            let _ = this.update(cx, |this, cx| {
                match result {
                    Ok(()) => {
                        if let LoadState::Loaded(profile) = &mut this.state {
                            profile.installation_id = installation_id;
                        }
                        this.launch_error = None;
                    }
                    Err(e) => this.launch_error = Some(e.to_string()),
                }
                cx.notify();
            });
        })
        .detach();
    }
}
