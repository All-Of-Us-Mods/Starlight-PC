//! Backend → frontend event bus.
//!
//! Services running on the tokio runtime publish progress / state events
//! through `publish()`. The GPUI layer subscribes via `subscribe()` and
//! forwards events into entity updates on the main thread.

use std::sync::LazyLock;
use tokio::sync::broadcast;

use crate::backend::services::bepinex_service::BepInExProgress;
use crate::backend::services::mod_download_service::ModDownloadProgress;
use crate::backend::state::game_runtime::GameStatePayload;

#[derive(Clone, Debug)]
pub enum BackendEvent {
    BepInExProgress(BepInExProgress),
    ModDownloadProgress(ModDownloadProgress),
    GameStateChanged(GameStatePayload),
    EpicLoginSuccess,
    EpicLoginError(String),
}

const CHANNEL_CAPACITY: usize = 256;

static EVENT_BUS: LazyLock<broadcast::Sender<BackendEvent>> = LazyLock::new(|| {
    let (tx, _rx) = broadcast::channel(CHANNEL_CAPACITY);
    tx
});

pub fn publish(event: BackendEvent) {
    // Errors only mean there are no live subscribers — fine, drop the event.
    let _ = EVENT_BUS.send(event);
}

pub fn subscribe() -> broadcast::Receiver<BackendEvent> {
    EVENT_BUS.subscribe()
}
