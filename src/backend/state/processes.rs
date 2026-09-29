//! Finding game processes through `/proc`. Wine moves the game out of our
//! process tree, so processes are recognised by their argv instead: launches
//! we spawn carry an instance tag, Steam's run under its reaper.

use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::os::unix::ffi::OsStrExt;

use crate::backend::services::installation_service::GAME_EXE_NAME;
use crate::backend::services::launch_service::STEAM_APP_ID;

fn processes() -> Vec<(i32, Vec<OsString>)> {
    std::fs::read_dir("/proc")
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            let pid = entry.file_name().to_str()?.parse().ok()?;
            Some((pid, read_nul_list(pid, "cmdline")?))
        })
        .collect()
}

fn read_nul_list(pid: i32, file: &str) -> Option<Vec<OsString>> {
    let bytes = std::fs::read(format!("/proc/{pid}/{file}")).ok()?;
    Some(
        bytes
            .split(|b| *b == 0)
            .filter(|part| !part.is_empty())
            .map(|part| OsStr::from_bytes(part).to_os_string())
            .collect(),
    )
}

fn parent_pid(pid: i32) -> Option<i32> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    // The command name before this may contain spaces or `)`.
    let after_name = &stat[stat.rfind(')')? + 1..];
    after_name.split_whitespace().nth(1)?.parse().ok()
}

/// Kills only the game process: wine and Proton then shut down on their own,
/// whereas killing them too strands wine's services.
fn kill_game(processes: impl IntoIterator<Item = (i32, Vec<OsString>)>) {
    let pids: Vec<String> = processes
        .into_iter()
        .filter(|(_, argv)| {
            argv.first()
                .is_some_and(|program| program.as_bytes().ends_with(GAME_EXE_NAME.as_bytes()))
        })
        .map(|(pid, _)| pid.to_string())
        .collect();
    if !pids.is_empty() {
        let _ = std::process::Command::new("kill")
            .arg("-KILL")
            .args(&pids)
            .status();
    }
}

pub fn kill_tagged_game(tag: &str) {
    kill_game(
        processes()
            .into_iter()
            .filter(|(_, argv)| argv.iter().any(|arg| arg == tag)),
    );
}

/// Steam runs every game as `reaper SteamLaunch AppId=<id> -- <command>`, and
/// the reaper lives exactly as long as the game. Steam also runs short-lived
/// reapers before the game (prefix setup), hence the exe check.
pub fn steam_reaper() -> Option<i32> {
    let app_id = format!("AppId={STEAM_APP_ID}");
    let game_exe = format!("/{GAME_EXE_NAME}");
    processes()
        .into_iter()
        .find(|(_, argv)| {
            let is = |i: usize, value: &str| argv.get(i).is_some_and(|arg| arg == value);
            argv.first()
                .is_some_and(|program| program.as_bytes().ends_with(b"/reaper"))
                && is(1, "SteamLaunch")
                && is(2, &app_id)
                && argv
                    .iter()
                    .any(|arg| arg.as_bytes().ends_with(game_exe.as_bytes()))
        })
        .map(|(pid, _)| pid)
}

fn steam_launch_tree() -> Vec<(i32, Vec<OsString>)> {
    let Some(reaper) = steam_reaper() else {
        return Vec::new();
    };
    let all = processes();
    let parents: HashMap<i32, i32> = all
        .iter()
        .filter_map(|(pid, _)| Some((*pid, parent_pid(*pid)?)))
        .collect();
    let under_reaper = |mut pid: i32| {
        while pid > 1 {
            if pid == reaper {
                return true;
            }
            let Some(parent) = parents.get(&pid) else {
                return false;
            };
            pid = *parent;
        }
        false
    };
    all.into_iter()
        .filter(|(pid, _)| under_reaper(*pid))
        .collect()
}

pub fn kill_steam_game() {
    kill_game(steam_launch_tree());
}

/// The Proton process Steam is running the game through, as it was started.
pub struct SteamProton {
    pub pid: i32,
    pub argv: Vec<OsString>,
    pub env: Vec<(OsString, OsString)>,
    /// Proton runs in the Steam Linux Runtime's container (and on NixOS or
    /// Flatpak, Steam's own sandbox). Another instance has to join it: it
    /// shares the running instance's wineserver, which can't reach processes
    /// in other user namespaces.
    pub sandboxed: bool,
    pub own_user_namespace: bool,
}

pub fn steam_proton() -> Option<SteamProton> {
    let (pid, argv) = steam_launch_tree().into_iter().find(|(_, argv)| {
        argv.iter()
            .take(2)
            .any(|arg| arg.as_bytes().ends_with(b"/proton"))
    })?;
    let env = read_nul_list(pid, "environ")?
        .into_iter()
        .filter_map(|entry| {
            let bytes = entry.as_bytes();
            let eq = bytes.iter().position(|b| *b == b'=')?;
            Some((
                OsStr::from_bytes(&bytes[..eq]).to_os_string(),
                OsStr::from_bytes(&bytes[eq + 1..]).to_os_string(),
            ))
        })
        .collect();
    let differs = |ns: &str| {
        std::fs::read_link(format!("/proc/{pid}/ns/{ns}")).ok()
            != std::fs::read_link(format!("/proc/self/ns/{ns}")).ok()
    };
    Some(SteamProton {
        pid,
        argv,
        env,
        sandboxed: differs("mnt"),
        own_user_namespace: differs("user"),
    })
}
