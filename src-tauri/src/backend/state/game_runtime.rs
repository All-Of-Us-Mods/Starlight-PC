use crate::backend::error::{AppError, AppResult};
use log::{debug, error, info, warn};
use std::collections::HashMap;
use std::process::Child;
use std::sync::{LazyLock, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Runtime};

struct TrackedGameProcess {
    child: Child,
    profile_id: Option<String>,
}

#[derive(Default)]
struct TrackedState {
    processes: Vec<TrackedGameProcess>,
    uwp_instances: Vec<Option<String>>,
}

static TRACKED_STATE: LazyLock<Mutex<TrackedState>> =
    LazyLock::new(|| Mutex::new(TrackedState::default()));

#[derive(Clone, serde::Serialize)]
pub struct GameStatePayload {
    pub running: bool,
    pub running_count: usize,
    pub stoppable_running_count: usize,
    pub profile_instance_counts: HashMap<String, usize>,
    pub stoppable_profile_instance_counts: HashMap<String, usize>,
}

fn reap_process(mut child: Child) {
    if let Err(e) = child.wait() {
        warn!("Failed to reap game process: {}", e);
    }
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

fn emit_state_snapshot<R: Runtime>(app: &AppHandle<R>, state: &TrackedState) {
    let payload = build_state_payload(state);
    let _ = app.emit("game-state-changed", payload);
}

fn prune_finished_processes(state: &mut TrackedState) {
    let mut i = 0;
    while i < state.processes.len() {
        match state.processes[i].child.try_wait() {
            Ok(Some(_)) => {
                let tracked = state.processes.swap_remove(i);
                reap_process(tracked.child);
            }
            Ok(None) => i += 1,
            Err(e) => {
                warn!("Failed to check tracked process state: {}", e);
                let tracked = state.processes.swap_remove(i);
                reap_process(tracked.child);
            }
        }
    }
}

fn monitor_game_process<R: Runtime>(app: AppHandle<R>, process_id: u32) {
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
                    reap_process(tracked.child);
                    emit_state_snapshot(&app, &state);
                    break;
                }
                Ok(None) => {}
                Err(e) => {
                    warn!("Failed to check game process state: {}", e);
                    let tracked = state.processes.swap_remove(index);
                    reap_process(tracked.child);
                    emit_state_snapshot(&app, &state);
                    break;
                }
            }
        }
    });
}

pub fn register_launched_process<R: Runtime>(
    app: AppHandle<R>,
    child: Child,
    profile_id: Option<String>,
) -> AppResult<()> {
    let process_id: u32;
    {
        let mut state = TRACKED_STATE
            .lock()
            .map_err(|_| AppError::state("Failed to acquire game process lock"))?;

        prune_finished_processes(&mut state);

        process_id = child.id();
        state
            .processes
            .push(TrackedGameProcess { child, profile_id });
        emit_state_snapshot(&app, &state);
    }

    monitor_game_process(app, process_id);
    Ok(())
}

fn stop_matching_processes<F>(state: &mut TrackedState, predicate: F) -> AppResult<usize>
where
    F: Fn(&TrackedGameProcess) -> bool,
{
    let mut stopped_count = 0;
    let mut i = 0;

    while i < state.processes.len() {
        if !predicate(&state.processes[i]) {
            i += 1;
            continue;
        }

        match state.processes[i].child.try_wait() {
            Ok(Some(_)) => {
                let tracked = state.processes.swap_remove(i);
                reap_process(tracked.child);
                continue;
            }
            Ok(None) => {}
            Err(e) => {
                warn!("Failed to check tracked process state before stop: {}", e);
                let tracked = state.processes.swap_remove(i);
                reap_process(tracked.child);
                continue;
            }
        }

        state.processes[i]
            .child
            .kill()
            .map_err(|e| AppError::process(format!("Failed to stop game process: {e}")))?;

        let tracked = state.processes.swap_remove(i);
        reap_process(tracked.child);
        stopped_count += 1;
    }

    Ok(stopped_count)
}

pub fn stop_profile_instances<R: Runtime>(
    app: &AppHandle<R>,
    profile_id: &str,
) -> AppResult<usize> {
    let mut state = TRACKED_STATE
        .lock()
        .map_err(|_| AppError::state("Failed to acquire game process lock"))?;

    prune_finished_processes(&mut state);
    let stopped_count = stop_matching_processes(&mut state, |tracked| {
        tracked.profile_id.as_deref() == Some(profile_id)
    })?;
    emit_state_snapshot(app, &state);
    Ok(stopped_count)
}

pub fn stop_all_tracked_instances<R: Runtime>(app: &AppHandle<R>) -> AppResult<usize> {
    let mut state = TRACKED_STATE
        .lock()
        .map_err(|_| AppError::state("Failed to acquire game process lock"))?;

    prune_finished_processes(&mut state);
    let stopped_count = stop_matching_processes(&mut state, |_| true)?;
    emit_state_snapshot(app, &state);
    Ok(stopped_count)
}

pub fn register_uwp_instance<R: Runtime>(
    app: &AppHandle<R>,
    profile_id: Option<String>,
) -> AppResult<()> {
    let mut state = TRACKED_STATE
        .lock()
        .map_err(|_| AppError::state("Failed to update game state"))?;
    state.uwp_instances.push(profile_id);
    emit_state_snapshot(app, &state);
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
