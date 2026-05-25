use gpui::*;
use std::path::PathBuf;

use crate::backend::api;
use crate::backend::services::profile_service::ProfileEntry;
use crate::theme::Theme;
use gpui_component::{Icon, IconName};

pub fn profile_icon(profile: &ProfileEntry, size: f32, theme: &Theme) -> AnyElement {
    let size_px = px(size);
    let common = |el: Div| {
        el.w(size_px)
            .h(size_px)
            .flex_none()
            .rounded_md()
            .overflow_hidden()
            .bg(theme.hover)
    };

    match profile.icon_mode.as_deref() {
        Some("custom") => {
            if let Some(ext) = profile
                .custom_icon_extension
                .as_deref()
                .filter(|s| !s.is_empty())
            {
                let path = PathBuf::from(&profile.path).join(format!("icon{ext}"));
                return common(div())
                    .child(img(path).w(size_px).h(size_px).object_fit(ObjectFit::Cover))
                    .into_any_element();
            }
        }
        Some("mod") => {
            if let Some(mod_id) = profile.icon_mod_id.as_deref().filter(|s| !s.is_empty()) {
                return common(div())
                    .child(
                        img(api::mod_thumbnail_url(mod_id))
                            .w(size_px)
                            .h(size_px)
                            .object_fit(ObjectFit::Cover),
                    )
                    .into_any_element();
            }
        }
        _ => {}
    }

    common(div())
        .flex()
        .items_center()
        .justify_center()
        .child(
            Icon::new(IconName::Inbox)
                .size(px(size * 0.55))
                .text_color(theme.text_muted),
        )
        .into_any_element()
}
