//! Asset source for the window.
//!
//! Loads our own app-specific SVGs (sidebar nav glyphs, logo) and
//! delegates everything else to `gpui_component_assets::Assets`, which
//! bundles the ~99 built-in icons gpui-component's `IconName` enum
//! references.

use std::borrow::Cow;

use gpui::{AssetSource, Result, SharedString};
use gpui_component_assets::Assets as ComponentAssets;

#[derive(Clone)]
pub struct EmbeddedAssets;

macro_rules! local_icons {
    ( $($path:literal),* $(,)? ) => {
        fn load_local(path: &str) -> Option<Cow<'static, [u8]>> {
            match path {
                $(
                    $path => Some(Cow::Borrowed(
                        include_bytes!(concat!("../assets/", $path)).as_slice(),
                    )),
                )*
                _ => None,
            }
        }
    };
}

local_icons!(
    "icons/home.svg",
    "icons/compass.svg",
    "icons/library.svg",
    "icons/download.svg",
    "icons/starlight.svg",
);

impl AssetSource for EmbeddedAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if let Some(bytes) = load_local(path) {
            return Ok(Some(bytes));
        }
        ComponentAssets.load(path)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        ComponentAssets.list(path)
    }
}
