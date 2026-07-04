//! Xbox / Microsoft Store launch support.
//!
//! The Microsoft Store build of Among Us is a UWP app: it can't be started by
//! running its exe directly (AppContainer sandboxing), and Doorstop can't be
//! handed `--doorstop-*` command-line args since Windows launches UWP apps
//! itself. So instead: resolve the app's package id, drop `winhttp.dll` +
//! a rewritten `doorstop_config.ini` into the install dir (same trick as the
//! Steam/Epic modded launch, just placed instead of passed as args), then
//! launch via `explorer shell:AppsFolder\{app_id}` protocol activation.
//!
//! Simplified from upstream: launched UWP instances aren't tracked as running
//! processes (there's no `Child` handle for them), so they don't show up in
//! the title bar's running/stoppable counts.

use crate::backend::error::{AppError, AppResult};
use log::debug;
use std::path::Path;
use std::process::Command;

/// Resolve the Start Menu app id for Among Us via PowerShell's `Get-StartApps`.
pub fn get_xbox_app_id() -> AppResult<String> {
    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "(Get-StartApps | Where-Object { $_.Name -like '*Among Us*' }).AppId",
        ])
        .output()
        .map_err(|e| AppError::process(format!("Failed to run PowerShell: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::process(format!(
            "PowerShell command failed: {stderr}"
        )));
    }

    let app_id = String::from_utf8_lossy(&output.stdout)
        .lines()
        .next()
        .unwrap_or("")
        .trim()
        .to_string();

    if app_id.is_empty() {
        return Err(AppError::validation(
            "Among Us not found in Microsoft Store apps. Please ensure the game is installed.",
        ));
    }

    Ok(app_id)
}

/// Copy the profile's `winhttp.dll` proxy and a `doorstop_config.ini`
/// (rewritten to point at this profile's BepInEx/coreclr) into the UWP
/// install dir.
pub fn prepare_xbox_launch(profile_path: &Path, game_dir: &Path) -> AppResult<()> {
    let src_dll = profile_path.join("winhttp.dll");
    let src_ini = profile_path.join("doorstop_config.ini");
    let dst_dll = game_dir.join("winhttp.dll");
    let dst_ini = game_dir.join("doorstop_config.ini");

    if !src_dll.exists() {
        return Err(AppError::validation(
            "winhttp.dll not found in profile. Please wait for BepInEx installation to complete.",
        ));
    }
    if !src_ini.exists() {
        return Err(AppError::validation(
            "doorstop_config.ini not found in profile. Please wait for BepInEx installation to complete.",
        ));
    }

    std::fs::copy(&src_dll, &dst_dll)?;

    let write_ini = || -> AppResult<()> {
        let target_assembly = profile_path
            .join("BepInEx")
            .join("core")
            .join("BepInEx.Unity.IL2CPP.dll");
        let coreclr_path = profile_path.join("dotnet").join("coreclr.dll");
        let target_assembly = target_assembly.to_string_lossy().replace('\\', "\\\\");
        let coreclr_path = coreclr_path.to_string_lossy().replace('\\', "\\\\");

        let ini_content = std::fs::read_to_string(&src_ini)?;
        let mut rewritten = String::new();
        for line in ini_content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("target_assembly") && trimmed.contains('=') {
                rewritten.push_str(&format!("target_assembly = \"{target_assembly}\"\n"));
            } else if trimmed.starts_with("coreclr_path") && trimmed.contains('=') {
                rewritten.push_str(&format!("coreclr_path = \"{coreclr_path}\"\n"));
            } else {
                rewritten.push_str(line);
                rewritten.push('\n');
            }
        }
        std::fs::write(&dst_ini, rewritten)?;
        Ok(())
    };

    if let Err(e) = write_ini() {
        let _ = std::fs::remove_file(&dst_dll);
        return Err(e);
    }

    Ok(())
}

/// Launch via Start Menu protocol activation — the only way to start a UWP
/// app outside of the Store/Start Menu itself.
pub fn launch_xbox(app_id: &str) -> AppResult<()> {
    let uri = format!("shell:AppsFolder\\{app_id}");
    debug!("Launching via: {uri}");

    Command::new("explorer")
        .arg(&uri)
        .spawn()
        .map_err(|e| AppError::process(format!("Failed to launch Xbox game: {e}")))?;

    Ok(())
}

/// Remove the doorstop files dropped by [`prepare_xbox_launch`], so a vanilla
/// launch doesn't accidentally load a previous profile's mods.
pub fn cleanup_xbox_files(game_dir: &Path) -> AppResult<()> {
    let dll_path = game_dir.join("winhttp.dll");
    let ini_path = game_dir.join("doorstop_config.ini");

    if dll_path.exists() {
        std::fs::remove_file(&dll_path)?;
    }
    if ini_path.exists() {
        std::fs::remove_file(&ini_path)?;
    }
    Ok(())
}
