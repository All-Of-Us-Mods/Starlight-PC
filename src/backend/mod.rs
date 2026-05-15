pub mod api;
pub mod directories;
pub mod error;
pub mod events;
pub mod runtime;
pub mod services;
pub mod state;

use gpui::App;
use log::debug;

/// Start the backend: warm the tokio runtime and attach a log-only event
/// subscriber. Views that care about specific events will add their own
/// subscribers via [`events::subscribe`].
pub fn init(cx: &mut App) {
    // Touch the lazy runtime so backend tasks can be spawned from anywhere.
    let _ = runtime::spawn(async {});

    let mut rx = events::subscribe();
    cx.background_executor()
        .spawn(async move {
            while let Ok(event) = rx.recv().await {
                debug!("backend event: {:?}", event);
            }
        })
        .detach();
}
