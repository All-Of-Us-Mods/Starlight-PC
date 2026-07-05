//! Epic Games launch support without account login.
//!
//! Epic's exchange-code auth can't be done headlessly anymore, so instead of
//! asking the user to paste tokens we borrow them from the Epic launcher
//! itself (the EpicGamesStarter trick): ask the launcher to start the game
//! via its `com.epicgames.launcher://` protocol, wait for the game process it
//! spawns, read the `-AUTH_LOGIN ... -AUTH_PASSWORD ...` arguments off its
//! command line, kill that instance, and reuse the arguments for our own
//! (modded) launch. Requires the Epic launcher to be installed and logged in,
//! but no manual token entry.

use crate::backend::error::{AppError, AppResult};
use log::{debug, info};
use std::collections::HashSet;
use std::os::windows::process::CommandExt as _;
use std::process::Command;
use std::time::{Duration, Instant};

/// Among Us' Epic catalog id (namespace:item:artifact, URL-encoded).
const EPIC_APP_ID: &str = "33956bcb55d4452d8c47e16b94e294bd%3A729a86a5146640a2ace9e8c595414c56%3A963137e4c29d4c79a81323b8fab03a40";

/// How long to wait for the Epic launcher to spawn the game. Cold launcher
/// starts (update check + login) can easily take tens of seconds.
const WAIT_TIMEOUT: Duration = Duration::from_secs(120);
const POLL_INTERVAL: Duration = Duration::from_millis(1000);

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// List running `Among Us.exe` processes as (pid, command line).
fn query_game_processes() -> AppResult<Vec<(u32, String)>> {
    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "Get-CimInstance Win32_Process -Filter \"Name='Among Us.exe'\" | ForEach-Object { \"$($_.ProcessId)|$($_.CommandLine)\" }",
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| AppError::process(format!("Failed to run PowerShell: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::process(format!(
            "Process query failed: {stderr}"
        )));
    }

    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let (pid, cmdline) = line.split_once('|')?;
            Some((pid.trim().parse().ok()?, cmdline.to_string()))
        })
        .collect())
}

fn kill_process(pid: u32) -> AppResult<()> {
    let status = Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/F"])
        .creation_flags(CREATE_NO_WINDOW)
        .status()
        .map_err(|e| AppError::process(format!("Failed to run taskkill: {e}")))?;
    if !status.success() {
        return Err(AppError::process(format!(
            "taskkill failed for pid {pid}"
        )));
    }
    Ok(())
}

/// Start the game through the Epic launcher, capture the auth arguments off
/// the instance it spawns, kill that instance, and return the argument string
/// (everything from `-AUTH_LOGIN` onwards). The `-AUTH_PASSWORD` exchange
/// code is single-use, so this runs once per launch.
pub fn acquire_launch_args() -> AppResult<String> {
    // Snapshot pids first so already-running (e.g. multi-instance modded)
    // copies aren't mistaken for the Epic-spawned one.
    let before: HashSet<u32> = query_game_processes()?
        .into_iter()
        .map(|(pid, _)| pid)
        .collect();

    let uri = format!("com.epicgames.launcher://apps/{EPIC_APP_ID}?action=launch&silent=true");
    info!("Epic launch: requesting game start via Epic launcher");
    Command::new("cmd")
        .args(["/C", "start", "", &uri])
        .creation_flags(CREATE_NO_WINDOW)
        .status()
        .map_err(|e| AppError::process(format!("Failed to open Epic launcher: {e}")))?;

    let deadline = Instant::now() + WAIT_TIMEOUT;
    while Instant::now() < deadline {
        for (pid, cmdline) in query_game_processes()? {
            if before.contains(&pid) {
                continue;
            }
            let Some(idx) = cmdline.find("-AUTH_LOGIN") else {
                continue;
            };
            let args = cmdline[idx..].trim().to_string();
            debug!("Epic launch: captured auth args from pid {pid}");
            kill_process(pid)?;
            return Ok(args);
        }
        std::thread::sleep(POLL_INTERVAL);
    }

    Err(AppError::process(
        "Timed out waiting for the Epic launcher to start the game. \
         Make sure the Epic Games Launcher is installed and logged in.",
    ))
}
