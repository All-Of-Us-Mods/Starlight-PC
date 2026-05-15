//! Shared tokio runtime for backend async work.
//!
//! GPUI has its own executor, but the ported backend uses tokio (reqwest,
//! `tokio::sync`, `tokio::task::spawn_blocking`). We run a single
//! multi-thread tokio runtime on its own pool and submit work to it from
//! the UI thread.

use std::future::Future;
use std::sync::LazyLock;
use tokio::runtime::{Builder, Runtime};
use tokio::task::JoinHandle;

static RUNTIME: LazyLock<Runtime> = LazyLock::new(|| {
    Builder::new_multi_thread()
        .enable_all()
        .thread_name("starlight-backend")
        .build()
        .expect("failed to build tokio runtime")
});

pub fn spawn<F>(future: F) -> JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    RUNTIME.spawn(future)
}

pub fn block_on<F: Future>(future: F) -> F::Output {
    RUNTIME.block_on(future)
}
