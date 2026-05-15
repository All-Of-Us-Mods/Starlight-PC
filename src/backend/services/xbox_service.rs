use crate::backend::error::{AppError, AppResult};
use crate::backend::state::game_runtime;
use log::{debug, info};
use std::path::PathBuf;
use std::process::Command;

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct XboxPrepareLaunchArgs {
    pub game_dir: String,
    pub profile_path: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct XboxLaunchArgs {
    pub app_id: String,
    pub profile_id: Option<String>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct XboxCleanupArgs {
    pub game_dir: String,
}

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

pub fn prepare_xbox_launch(args: XboxPrepareLaunchArgs) -> AppResult<()> {
    let game_dir = PathBuf::from(&args.game_dir);
    let profile_path = PathBuf::from(&args.profile_path);

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

    let cleanup_dll = || {
        if dst_dll.exists() {
            let _ = std::fs::remove_file(&dst_dll);
        }
    };

    let ini_content = std::fs::read_to_string(&src_ini).map_err(|e| {
        cleanup_dll();
        AppError::process(format!("Failed to read doorstop_config.ini: {e}"))
    })?;

    let target_assembly = profile_path
        .join("BepInEx")
        .join("core")
        .join("BepInEx.Unity.IL2CPP.dll");
    let coreclr_path = profile_path.join("dotnet").join("coreclr.dll");

    let target_assembly_str = target_assembly.to_string_lossy().replace('\\', "\\\\");
    let coreclr_path_str = coreclr_path.to_string_lossy().replace('\\', "\\\\");

    let mut modified_content = String::new();
    for line in ini_content.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('#')
            && !trimmed.starts_with(';')
            && trimmed.starts_with("target_assembly")
            && trimmed.contains('=')
        {
            modified_content.push_str(&format!("target_assembly = \"{}\"\n", target_assembly_str));
        } else if !trimmed.starts_with('#')
            && !trimmed.starts_with(';')
            && trimmed.starts_with("coreclr_path")
            && trimmed.contains('=')
        {
            modified_content.push_str(&format!("coreclr_path = \"{}\"\n", coreclr_path_str));
        } else {
            modified_content.push_str(line);
            modified_content.push('\n');
        }
    }

    std::fs::write(&dst_ini, modified_content).map_err(|e| {
        cleanup_dll();
        AppError::process(format!("Failed to write doorstop_config.ini: {e}"))
    })?;

    Ok(())
}

pub fn launch_xbox(args: XboxLaunchArgs) -> AppResult<()> {
    let uri = format!("shell:AppsFolder\\{}", args.app_id);
    debug!("Launching via: {}", uri);

    Command::new("explorer")
        .arg(&uri)
        .spawn()
        .map_err(|e| AppError::process(format!("Failed to launch Xbox game: {e}")))?;

    game_runtime::register_uwp_instance(args.profile_id)?;
    info!("Xbox game launched (no process monitoring available for UWP apps)");
    Ok(())
}

pub fn cleanup_xbox_files(args: XboxCleanupArgs) -> AppResult<()> {
    let game_dir = PathBuf::from(&args.game_dir);
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
