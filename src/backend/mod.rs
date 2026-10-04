pub mod api;
pub mod binary;
pub mod deeplink;
pub mod directories;
pub mod error;
pub mod events;
pub mod services;
pub mod single_instance;
pub mod state;
#[cfg(test)]
pub(crate) mod test_support;

use gpui_kit::App;
use log::debug;

/// Attach a default log-only subscriber to the event bus. Views that care
/// about specific events register their own subscribers via [`events::subscribe`].
pub fn init(cx: &mut App) {
    // Instance copies and temporary profiles belong to the process that
    // launched from them, so any still on disk now are leftovers from a crash.
    cx.background_executor()
        .spawn(async {
            services::profile_instance_service::cleanup_stale_copies();
            if let Err(e) = services::profile_service::delete_temporary_profiles() {
                log::warn!("Failed to clean up temporary profiles: {e}");
            }
        })
        .detach();

    let mut rx = events::subscribe();
    cx.background_executor()
        .spawn(async move {
            while let Ok(event) = rx.recv().await {
                debug!("backend event: {:?}", event);
            }
        })
        .detach();
}
