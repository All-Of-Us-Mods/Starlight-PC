use crate::backend::error::{AppError, AppResult};
use crate::backend::services::profile_service;
use log::{debug, error, info, warn};
use std::collections::HashMap;
use std::process::Child;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

struct TrackedGameProcess {
    child: Child,
    profile_id: Option<String>,
    launched_at: Instant,
}

#[derive(Default)]
struct TrackedState {
    processes: Vec<TrackedGameProcess>,
    uwp_instances: Vec<Option<String>>,
}

static TRACKED_STATE: LazyLock<Mutex<TrackedState>> =
    LazyLock::new(|| Mutex::new(TrackedState::default()));

#[derive(Clone, Debug, serde::Serialize)]
pub struct GameStatePayload {
    pub running: bool,
    pub running_count: usize,
    pub stoppable_running_count: usize,
    pub profile_instance_counts: HashMap<String, usize>,
    pub stoppable_profile_instance_counts: HashMap<String, usize>,
}

// Proton/Steam reparent the actual game outside our process tree, so we can't
// rely on PID/pgid. Every relevant process (wrapper, proton, wine, the game)
// carries an identifying substring in its cmdline — the profile id (via the
// doorstop args we pass for modded launches) or `Among Us.exe` (passed as
// the game-exe argument for any launch) — so match on that.
#[cfg(target_os = "linux")]
fn kill_by_cmdline_substring(substring: &str) {
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return;
    };
    for entry in entries.flatten() {
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|s| s.parse::<i32>().ok())
        else {
            continue;
        };
        let Ok(cmdline) = std::fs::read(format!("/proc/{pid}/cmdline")) else {
            continue;
        };
        if String::from_utf8_lossy(&cmdline).contains(substring) {
            let _ = std::process::Command::new("kill")
                .args(["-KILL", &pid.to_string()])
                .status();
        }
    }
}

fn reap_process(mut child: Child) {
    if let Err(e) = child.wait() {
        warn!("Failed to reap game process: {}", e);
    }
}

/// Reap a tracked process and record its session against the owning profile.
/// Play-time persistence is offloaded to a detached thread so callers don't
/// block (and don't hold the tracked-state lock) during disk I/O.
fn reap_and_record(tracked: TrackedGameProcess) {
    let TrackedGameProcess {
        child,
        profile_id,
        launched_at,
    } = tracked;
    reap_process(child);
    let Some(id) = profile_id else { return };
    let duration_ms = launched_at.elapsed().as_millis().min(i64::MAX as u128) as i64;
    if duration_ms <= 0 {
        return;
    }
    std::thread::spawn(move || {
        if let Err(e) = profile_service::add_play_time(&id, duration_ms) {
            warn!("add_play_time failed for profile {id}: {e}");
        }
        crate::backend::events::publish(crate::backend::events::BackendEvent::ProfileStatsUpdated(
            id,
        ));
    });
}

fn build_state_payload(state: &TrackedState) -> GameStatePayload {
    let mut profile_instance_counts = HashMap::new();
    let mut stoppable_profile_instance_counts = HashMap::new();
    for tracked in &state.processes {
        if let Some(profile_id) = &tracked.profile_id {
            *profile_instance_counts
                .entry(profile_id.clone())
                .or_insert(0) += 1;
            *stoppable_profile_instance_counts
                .entry(profile_id.clone())
                .or_insert(0) += 1;
        }
    }
    for profile_id in state.uwp_instances.iter().flatten() {
        *profile_instance_counts
            .entry(profile_id.clone())
            .or_insert(0) += 1;
    }

    let running_count = state.processes.len() + state.uwp_instances.len();
    GameStatePayload {
        running: running_count > 0,
        running_count,
        stoppable_running_count: state.processes.len(),
        profile_instance_counts,
        stoppable_profile_instance_counts,
    }
}

fn emit_state_snapshot(state: &TrackedState) {
    let payload = build_state_payload(state);
    crate::backend::events::publish(crate::backend::events::BackendEvent::GameStateChanged(
        payload,
    ));
}

fn prune_finished_processes(state: &mut TrackedState) {
    let mut i = 0;
    while i < state.processes.len() {
        match state.processes[i].child.try_wait() {
            Ok(Some(_)) => {
                let tracked = state.processes.swap_remove(i);
                reap_and_record(tracked);
            }
            Ok(None) => i += 1,
            Err(e) => {
                warn!("Failed to check tracked process state: {}", e);
                let tracked = state.processes.swap_remove(i);
                reap_and_record(tracked);
            }
        }
    }
}

fn monitor_game_process(process_id: u32) {
    std::thread::spawn(move || {
        info!("Monitoring game process state");
        loop {
            std::thread::sleep(Duration::from_millis(500));

            let Ok(mut state) = TRACKED_STATE.lock() else {
                error!("Failed to acquire game process lock");
                break;
            };

            let Some(index) = state
                .processes
                .iter()
                .position(|tracked| tracked.child.id() == process_id)
            else {
                debug!("Monitored process no longer available");
                break;
            };

            match state.processes[index].child.try_wait() {
                Ok(Some(status)) => {
                    info!("Game process exited with status: {:?}", status);
                    let tracked = state.processes.swap_remove(index);
                    emit_state_snapshot(&state);
                    drop(state);
                    reap_and_record(tracked);
                    break;
                }
                Ok(None) => {}
                Err(e) => {
                    warn!("Failed to check game process state: {}", e);
                    let tracked = state.processes.swap_remove(index);
                    emit_state_snapshot(&state);
                    drop(state);
                    reap_and_record(tracked);
                    break;
                }
            }
        }
    });
}

pub fn register_launched_process(child: Child, profile_id: Option<String>) -> AppResult<()> {
    if let Some(id) = profile_id.as_deref() {
        if let Err(e) = profile_service::update_last_launched(id) {
            warn!("update_last_launched failed for profile {id}: {e}");
        } else {
            crate::backend::events::publish(
                crate::backend::events::BackendEvent::ProfileStatsUpdated(id.to_string()),
            );
        }
    }

    let process_id: u32;
    {
        let mut state = TRACKED_STATE
            .lock()
            .map_err(|_| AppError::state("Failed to acquire game process lock"))?;

        prune_finished_processes(&mut state);

        process_id = child.id();
        state.processes.push(TrackedGameProcess {
            child,
            profile_id,
            launched_at: Instant::now(),
        });
        emit_state_snapshot(&state);
    }

    monitor_game_process(process_id);
    Ok(())
}

fn stop_matching_processes<F>(state: &mut TrackedState, predicate: F) -> AppResult<usize>
where
    F: Fn(&TrackedGameProcess) -> bool,
{
    let mut stopped_count = 0;
    #[cfg_attr(target_os = "linux", allow(unused_mut))]
    let mut errors: Vec<String> = Vec::new();
    let mut i = 0;

    while i < state.processes.len() {
        if !predicate(&state.processes[i]) {
            i += 1;
            continue;
        }

        match state.processes[i].child.try_wait() {
            Ok(Some(_)) => {
                let tracked = state.processes.swap_remove(i);
                reap_and_record(tracked);
                continue;
            }
            Ok(None) => {}
            Err(e) => {
                warn!("Failed to check tracked process state before stop: {}", e);
                let tracked = state.processes.swap_remove(i);
                reap_and_record(tracked);
                continue;
            }
        }

        // On Linux, kill_by_cmdline_substring (called before this loop) already SIGKILLed
        // every matching cmdline; just reap. Other platforms kill the tracked
        // child directly.
        #[cfg(not(target_os = "linux"))]
        {
            let pid = state.processes[i].child.id();
            if let Err(e) = state.processes[i].child.kill() {
                warn!("Failed to kill game process: {}", e);
                errors.push(format!("Failed to stop game process {pid}: {e}"));
                i += 1;
                continue;
            }
        }

        let tracked = state.processes.swap_remove(i);
        reap_and_record(tracked);
        stopped_count += 1;
    }

    if !errors.is_empty() {
        return Err(AppError::process(errors.join("; ")));
    }

    Ok(stopped_count)
}

pub fn stop_profile_instances(profile_id: &str) -> AppResult<usize> {
    #[cfg(target_os = "linux")]
    kill_by_cmdline_substring(profile_id);

    let mut state = TRACKED_STATE
        .lock()
        .map_err(|_| AppError::state("Failed to acquire game process lock"))?;

    prune_finished_processes(&mut state);
    let stop_result = stop_matching_processes(&mut state, |tracked| {
        tracked.profile_id.as_deref() == Some(profile_id)
    });
    emit_state_snapshot(&state);
    stop_result
}

pub fn current_state() -> GameStatePayload {
    let state = TRACKED_STATE.lock().unwrap_or_else(|e| e.into_inner());
    build_state_payload(&state)
}

pub fn stop_all_tracked_instances() -> AppResult<usize> {
    // SIGKILL anything that smells like Among Us first — same trick as the
    // per-profile stop, just with a broader marker so vanilla launches (no
    // profile id in their cmdline) get caught too.
    #[cfg(target_os = "linux")]
    kill_by_cmdline_substring("Among Us.exe");

    let mut state = TRACKED_STATE
        .lock()
        .map_err(|_| AppError::state("Failed to acquire game process lock"))?;

    prune_finished_processes(&mut state);
    let stop_result = stop_matching_processes(&mut state, |_| true);
    emit_state_snapshot(&state);
    stop_result
}

#[allow(dead_code)] // planned: used by xbox_service when Xbox launch lands
pub fn register_uwp_instance(profile_id: Option<String>) -> AppResult<()> {
    let mut state = TRACKED_STATE
        .lock()
        .map_err(|_| AppError::state("Failed to update game state"))?;
    state.uwp_instances.push(profile_id);
    emit_state_snapshot(&state);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payload_aggregates_profile_counts() {
        let mut state = TrackedState::default();
        state.uwp_instances.push(Some("p1".to_string()));
        state.uwp_instances.push(Some("p1".to_string()));
        state.uwp_instances.push(Some("p2".to_string()));

        let payload = build_state_payload(&state);
        assert!(payload.running);
        assert_eq!(payload.running_count, 3);
        assert_eq!(payload.stoppable_running_count, 0);
        assert_eq!(payload.profile_instance_counts.get("p1"), Some(&2));
        assert_eq!(payload.profile_instance_counts.get("p2"), Some(&1));
        assert!(payload.stoppable_profile_instance_counts.is_empty());
    }
}
