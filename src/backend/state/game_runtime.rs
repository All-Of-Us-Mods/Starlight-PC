//! The game instances Starlight launched (and modded launches on their way),
//! and a poller that settles them (play time, instance copy, temporary
//! profile) once they exit.

use crate::backend::services::{profile_instance_service, profile_service};
use log::{info, warn};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::process::Child;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{LazyLock, Mutex, Once};
use std::time::{Duration, Instant};

#[cfg(target_os = "linux")]
pub use super::processes::steam_proton;

/// Slot 0 runs from the profile itself, higher slots from a throwaway copy
/// (`temporary_dir`) so concurrent instances don't share BepInEx state.
#[derive(Default)]
pub struct LaunchInstance {
    pub slot: usize,
    pub temporary_dir: Option<PathBuf>,
}

enum Process {
    /// The game, or the wine/Proton wrapper that lives as long as it.
    Spawned(Child),
    /// Handed to the Steam client; `seen` once its reaper shows up.
    #[cfg(target_os = "linux")]
    Steam { seen: bool },
}

struct Instance {
    id: u64,
    profile_id: Option<String>,
    launched_at: Instant,
    launch: LaunchInstance,
    process: Process,
}

impl Instance {
    fn is_running(&mut self) -> bool {
        match &mut self.process {
            Process::Spawned(child) => matches!(child.try_wait(), Ok(None)),
            #[cfg(target_os = "linux")]
            Process::Steam { seen } => {
                const STARTUP_GRACE: Duration = Duration::from_secs(120);
                if super::processes::steam_reaper().is_some() {
                    *seen = true;
                    true
                } else {
                    !*seen && self.launched_at.elapsed() < STARTUP_GRACE
                }
            }
        }
    }

    fn kill(&mut self) {
        match &mut self.process {
            Process::Spawned(child) => {
                #[cfg(target_os = "linux")]
                super::processes::kill_tagged_game(&instance_arg(self.id));
                let _ = child.kill();
            }
            #[cfg(target_os = "linux")]
            Process::Steam { .. } => super::processes::kill_steam_game(),
        }
    }

    fn finish(mut self) {
        match &mut self.process {
            Process::Spawned(child) => {
                if let Err(e) = child.wait() {
                    warn!("Failed to reap game process: {e}");
                }
            }
            #[cfg(target_os = "linux")]
            Process::Steam { .. } => {}
        }
        settle_profile(self.profile_id, self.launched_at);
        if let Some(directory) = self.launch.temporary_dir {
            std::thread::spawn(move || profile_instance_service::release(&directory));
        }
    }
}

static INSTANCES: LazyLock<Mutex<Vec<Instance>>> = LazyLock::new(Mutex::default);
/// Modded launches requested but not yet running or failed, by key and
/// profile id. Locked after `INSTANCES` when both are needed.
static PENDING: Mutex<Vec<(u64, String)>> = Mutex::new(Vec::new());
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

fn instances() -> std::sync::MutexGuard<'static, Vec<Instance>> {
    INSTANCES.lock().unwrap_or_else(|e| e.into_inner())
}

fn pending() -> std::sync::MutexGuard<'static, Vec<(u64, String)>> {
    PENDING.lock().unwrap_or_else(|e| e.into_inner())
}

/// A modded launch from request until it runs or fails. Counts toward its
/// profile in [`GameStatePayload`] so Stop shows at once; Stop cancels it.
pub struct PendingLaunch(u64);

pub fn begin_launch(profile_id: &str) -> PendingLaunch {
    let key = next_instance_id();
    pending().push((key, profile_id.to_string()));
    publish(&instances());
    PendingLaunch(key)
}

impl PendingLaunch {
    pub fn cancelled(&self) -> bool {
        !pending().iter().any(|(key, _)| *key == self.0)
    }
}

impl Drop for PendingLaunch {
    fn drop(&mut self) {
        let instances = instances();
        let mut pending = pending();
        let before = pending.len();
        pending.retain(|(key, _)| *key != self.0);
        let removed = pending.len() != before;
        drop(pending);
        if removed {
            publish(&instances);
        }
    }
}

pub fn next_instance_id() -> u64 {
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

/// Tags a spawned launch's processes so Stop can find them once wine has
/// moved them out of our process tree. Unity ignores unknown arguments. The
/// session part keeps games that outlived an earlier Starlight run apart.
#[cfg(target_os = "linux")]
pub fn instance_arg(id: u64) -> String {
    static SESSION: LazyLock<String> = LazyLock::new(|| uuid::Uuid::new_v4().simple().to_string());
    format!("--starlight-instance={}-{id}", *SESSION)
}

#[derive(Clone, Debug, Default)]
pub struct GameStatePayload {
    /// Running instances.
    pub running_count: usize,
    /// Running instances plus pending launches, per profile.
    pub profile_instance_counts: HashMap<String, usize>,
}

impl GameStatePayload {
    pub fn profile_count(&self, profile_id: &str) -> usize {
        self.profile_instance_counts
            .get(profile_id)
            .copied()
            .unwrap_or(0)
    }
}

pub fn current_state() -> GameStatePayload {
    state_of(&instances())
}

fn state_of(instances: &[Instance]) -> GameStatePayload {
    let mut profile_instance_counts = HashMap::new();
    let running = instances.iter().filter_map(|i| i.profile_id.clone());
    for profile_id in running.chain(pending().iter().map(|(_, id)| id.clone())) {
        *profile_instance_counts.entry(profile_id).or_insert(0) += 1;
    }
    GameStatePayload {
        running_count: instances.len(),
        profile_instance_counts,
    }
}

fn publish(instances: &[Instance]) {
    crate::backend::events::publish(crate::backend::events::BackendEvent::GameStateChanged(
        state_of(instances),
    ));
}

pub fn used_instance_slots(profile_id: &str) -> HashSet<usize> {
    instances()
        .iter()
        .filter(|i| i.profile_id.as_deref() == Some(profile_id))
        .map(|i| i.launch.slot)
        .collect()
}

fn register(profile_id: Option<String>, launch: LaunchInstance, id: u64, process: Process) {
    mark_launched(profile_id.as_deref());
    let mut instances = instances();
    instances.push(Instance {
        id,
        profile_id,
        launched_at: Instant::now(),
        launch,
        process,
    });
    publish(&instances);
    drop(instances);
    start_poller();
}

pub fn register_launched_process(
    id: u64,
    child: Child,
    profile_id: Option<String>,
    launch: LaunchInstance,
) {
    register(profile_id, launch, id, Process::Spawned(child));
}

/// Whether a launch handed to the Steam client is starting or running.
#[cfg(target_os = "linux")]
pub fn steam_launch_pending() -> bool {
    instances()
        .iter()
        .any(|i| matches!(i.process, Process::Steam { .. }))
}

/// Steam runs one instance of the game itself, so this replaces any earlier one.
#[cfg(target_os = "linux")]
pub fn register_steam_launch(profile_id: Option<String>) {
    instances().retain(|i| !matches!(i.process, Process::Steam { .. }));
    register(
        profile_id,
        LaunchInstance::default(),
        next_instance_id(),
        Process::Steam { seen: false },
    );
}

fn start_poller() {
    static POLLER: Once = Once::new();
    POLLER.call_once(|| {
        std::thread::spawn(|| {
            loop {
                std::thread::sleep(Duration::from_millis(500));
                let mut instances = instances();
                let exited: Vec<Instance> = instances.extract_if(.., |i| !i.is_running()).collect();
                if exited.is_empty() {
                    continue;
                }
                publish(&instances);
                drop(instances);
                for instance in exited {
                    info!("game instance {} exited", instance.id);
                    instance.finish();
                }
            }
        });
    });
}

/// Cancel pending launches and stop instances of `profile_id`, or of every
/// profile (and vanilla) when `None`.
fn stop_where(profile_id: Option<&str>) -> usize {
    let matches = |id: Option<&str>| profile_id.is_none_or(|p| id == Some(p));
    let mut instances = instances();
    pending().retain(|(_, id)| !matches(Some(id)));
    let mut stopped: Vec<Instance> = instances
        .extract_if(.., |i| matches(i.profile_id.as_deref()))
        .collect();
    publish(&instances);
    drop(instances);
    for instance in &mut stopped {
        instance.kill();
    }
    let count = stopped.len();
    stopped.into_iter().for_each(Instance::finish);
    count
}

pub fn stop_profile_instances(profile_id: &str) -> usize {
    stop_where(Some(profile_id))
}

pub fn stop_all_tracked_instances() -> usize {
    stop_where(None)
}

fn mark_launched(profile_id: Option<&str>) {
    let Some(id) = profile_id else { return };
    if let Err(e) = profile_service::update_last_launched(id) {
        warn!("update_last_launched failed for profile {id}: {e}");
    }
}

/// Credit an exited instance's play time to its profile, or delete the
/// profile if it's temporary and nothing else is running from it.
fn settle_profile(profile_id: Option<String>, launched_at: Instant) {
    let Some(id) = profile_id else { return };
    let duration_ms = launched_at.elapsed().as_millis().min(i64::MAX as u128) as i64;
    std::thread::spawn(move || {
        if let Ok(Some(profile)) = profile_service::get_profile_by_id(&id)
            && profile.temporary
        {
            if !current_state().profile_instance_counts.contains_key(&id)
                && let Err(e) = profile_service::delete_profile(&id)
            {
                warn!("failed to delete temporary profile {id}: {e}");
            }
            return;
        }
        if let Err(e) = profile_service::add_play_time(&id, duration_ms) {
            warn!("add_play_time failed for profile {id}: {e}");
        }
    });
}
