//! Embedded asset source for icons.
//!
//! All SVGs are baked into the binary via `include_bytes!` so the app
//! has no runtime dependency on its working directory.

use std::borrow::Cow;

use gpui::{AssetSource, Result, SharedString};

#[derive(Clone)]
pub struct EmbeddedAssets;

macro_rules! icons {
    ( $($path:literal),* $(,)? ) => {
        impl AssetSource for EmbeddedAssets {
            fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
                Ok(match path {
                    $(
                        $path => Some(Cow::Borrowed(
                            include_bytes!(concat!("../assets/", $path)).as_slice(),
                        )),
                    )*
                    _ => None,
                })
            }

            fn list(&self, _: &str) -> Result<Vec<SharedString>> {
                Ok(vec![])
            }
        }
    };
}

icons!(
    "icons/home.svg",
    "icons/compass.svg",
    "icons/library.svg",
    "icons/download.svg",
    "icons/starlight.svg",
);
