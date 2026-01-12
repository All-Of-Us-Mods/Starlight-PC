use crate::utils::epic_api::{self, EpicApi};
use log::{debug, error, info, warn};
use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::{LazyLock, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Runtime};

static GAME_PROCESS: LazyLock<Mutex<Option<Child>>> = LazyLock::new(|| Mutex::new(None));

#[derive(Clone, serde::Serialize)]
pub struct GameStatePayload {
    pub running: bool,
}

/// Monitors the game process and emits state changes.
fn monitor_game_process<R: Runtime>(app: AppHandle<R>) {
    std::thread::spawn(move || {
        let _ = app.emit("game-state-changed", GameStatePayload { running: true });
        info!("Game process started, monitoring state");

        loop {
            std::thread::sleep(Duration::from_millis(500));

            let Ok(mut guard) = GAME_PROCESS.lock() else {
                error!("Failed to acquire game process lock");
                break;
            };

            match guard.as_mut().and_then(|c| c.try_wait().ok()) {
                Some(Some(status)) => {
                    info!("Game process exited with status: {:?}", status);
                    *guard = None;
                    break;
                }
                None => {
                    debug!("Game process no longer available");
                    *guard = None;
                    break;
                }
                Some(None) => {}
            }
        }

        let _ = app.emit("game-state-changed", GameStatePayload { running: false });
        info!("Game state changed to not running");
    });
}

#[cfg(windows)]
fn set_dll_directory(path: &str) -> Result<(), String> {
    use windows::Win32::System::LibraryLoader::SetDllDirectoryW;
    use windows::core::PCWSTR;

    debug!("Setting DLL directory to: {}", path);
    let wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe { SetDllDirectoryW(PCWSTR(wide.as_ptr())) }.map_err(|e| {
        error!("SetDllDirectory failed: {}", e);
        format!("SetDllDirectory failed: {e}")
    })
}

fn launch<R: Runtime>(app: AppHandle<R>, mut cmd: Command) -> Result<(), String> {
    {
        let mut guard = GAME_PROCESS.lock().unwrap();

        if guard
            .as_mut()
            .is_some_and(|c| c.try_wait().ok().flatten().is_none())
        {
            warn!("Attempted to launch game while already running");
            return Err("Game is already running".into());
        }

        info!("Launching game process");
        let child = cmd.spawn().map_err(|e| {
            error!("Failed to launch game: {}", e);
            format!("Failed to launch game: {e}")
        })?;
        *guard = Some(child);
    }

    monitor_game_process(app);

    Ok(())
}

#[tauri::command]
pub async fn launch_modded<R: Runtime>(
    app: AppHandle<R>,
    game_exe: String,
    _profile_path: String,
    bepinex_dll: String,
    dotnet_dir: String,
    coreclr_path: String,
    platform: String,
) -> Result<(), String> {
    info!("launch_modded: game_exe={}", game_exe);
    debug!(
        "launch_modded: profile_path={}, bepinex_dll={}, dotnet_dir={}, coreclr_path={}",
        _profile_path, bepinex_dll, dotnet_dir, coreclr_path
    );

    let game_dir = PathBuf::from(&game_exe);
    let game_dir = game_dir.parent().ok_or_else(|| {
        error!("Invalid game path: {}", game_exe);
        "Invalid game path"
    })?;

    #[cfg(windows)]
    set_dll_directory(&_profile_path)?;

    let mut cmd = Command::new(&game_exe);
    cmd.current_dir(game_dir)
        .args(["--doorstop-enabled", "true"])
        .args(["--doorstop-target-assembly", &bepinex_dll])
        .args(["--doorstop-clr-corlib-dir", &dotnet_dir])
        .args(["--doorstop-clr-runtime-coreclr-path", &coreclr_path]);

    if platform == "epic"
        && let Some(session) = epic_api::load_session()
    {
        info!("Epic session found, obtaining game token");
        let api = EpicApi::new()?;
        match api.get_game_token(&session).await {
            Ok(launch_token) => {
                debug!("Epic game token obtained successfully");
                cmd.arg(format!("-AUTH_PASSWORD={}", launch_token));
            }
            Err(e) => {
                warn!("Failed to get Epic game token: {}", e);
            }
        }
    }

    launch(app, cmd)
}

#[tauri::command]
pub async fn launch_vanilla<R: Runtime>(
    app: AppHandle<R>,
    game_exe: String,
    platform: String,
) -> Result<(), String> {
    info!("launch_vanilla: game_exe={}", game_exe);
    let mut cmd = Command::new(&game_exe);

    if platform == "epic"
        && let Some(session) = epic_api::load_session()
    {
        info!("Epic session found, obtaining game token");
        let api = EpicApi::new()?;
        match api.get_game_token(&session).await {
            Ok(launch_token) => {
                debug!("Epic game token obtained successfully");
                cmd.arg(format!("-AUTH_PASSWORD={}", launch_token));
            }
            Err(e) => {
                warn!("Failed to get Epic game token: {}", e);
            }
        }
    }

    launch(app, cmd)
}

// =============================================================================
// Xbox / Microsoft Store Commands
// =============================================================================

/// Gets the Xbox AppUserModelId for Among Us by querying installed Start Menu apps.
#[tauri::command]
pub async fn get_xbox_app_id() -> Result<String, String> {
    info!("get_xbox_app_id: Querying for Among Us in Start Apps");

    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "(Get-StartApps | Where-Object { $_.Name -like '*Among Us*' }).AppId",
        ])
        .output()
        .map_err(|e| {
            error!("Failed to run PowerShell: {}", e);
            format!("Failed to run PowerShell: {e}")
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!("PowerShell command failed: {}", stderr);
        return Err(format!("PowerShell command failed: {stderr}"));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let app_id = stdout.lines().next().unwrap_or("").trim().to_string();

    if app_id.is_empty() {
        error!("Among Us not found in Microsoft Store apps");
        return Err(
            "Among Us not found in Microsoft Store apps. Please ensure the game is installed."
                .to_string(),
        );
    }

    info!("Found Xbox AppUserModelId: {}", app_id);
    Ok(app_id)
}

#[cfg(not(windows))]
#[tauri::command]
pub async fn get_xbox_app_id() -> Result<String, String> {
    Err("Xbox game launching is only supported on Windows".to_string())
}

/// Prepares an Xbox launch by copying doorstop files and modifying the config.
#[tauri::command]
pub async fn prepare_xbox_launch(game_dir: String, profile_path: String) -> Result<(), String> {
    info!(
        "prepare_xbox_launch: game_dir={}, profile_path={}",
        game_dir, profile_path
    );

    let game_dir = PathBuf::from(&game_dir);
    let profile_path = PathBuf::from(&profile_path);

    // Source files in profile root
    let src_dll = profile_path.join("winhttp.dll");
    let src_ini = profile_path.join("doorstop_config.ini");

    // Destination files in game directory
    let dst_dll = game_dir.join("winhttp.dll");
    let dst_ini = game_dir.join("doorstop_config.ini");

    // Check source files exist
    if !src_dll.exists() {
        error!("winhttp.dll not found at: {:?}", src_dll);
        return Err(
            "winhttp.dll not found in profile. Please wait for BepInEx installation to complete."
                .to_string(),
        );
    }
    if !src_ini.exists() {
        error!("doorstop_config.ini not found at: {:?}", src_ini);
        return Err(
            "doorstop_config.ini not found in profile. Please wait for BepInEx installation to complete."
                .to_string(),
        );
    }

    // Copy winhttp.dll
    debug!("Copying winhttp.dll to game directory");
    std::fs::copy(&src_dll, &dst_dll).map_err(|e| {
        error!("Failed to copy winhttp.dll: {}", e);
        format!("Failed to copy winhttp.dll: {e}")
    })?;

    // Helper to clean up on failure
    let cleanup_dll = || {
        if dst_dll.exists() {
            let _ = std::fs::remove_file(&dst_dll);
        }
    };

    // Read and modify doorstop_config.ini
    debug!("Reading doorstop_config.ini");
    let ini_content = std::fs::read_to_string(&src_ini).map_err(|e| {
        error!("Failed to read doorstop_config.ini: {}", e);
        cleanup_dll();
        format!("Failed to read doorstop_config.ini: {e}")
    })?;

    // Build absolute paths for the profile
    let target_assembly = profile_path
        .join("BepInEx")
        .join("core")
        .join("BepInEx.Unity.IL2CPP.dll");
    let coreclr_path = profile_path.join("dotnet").join("coreclr.dll");

    // Escape backslashes for INI file (Windows paths)
    let target_assembly_str = target_assembly.to_string_lossy().replace('\\', "\\\\");
    let coreclr_path_str = coreclr_path.to_string_lossy().replace('\\', "\\\\");

    // Replace the values in the INI content (skip comments starting with # or ;)
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

    // Write modified INI to game directory
    debug!("Writing modified doorstop_config.ini to game directory");
    std::fs::write(&dst_ini, modified_content).map_err(|e| {
        error!("Failed to write doorstop_config.ini: {}", e);
        cleanup_dll();
        format!("Failed to write doorstop_config.ini: {e}")
    })?;

    info!("Xbox launch preparation complete");
    Ok(())
}

#[cfg(not(windows))]
#[tauri::command]
pub async fn prepare_xbox_launch() -> Result<String, String> {
    Err("Xbox game launching is only supported on Windows".to_string())
}

/// Launches the Xbox/Microsoft Store version of Among Us.
/// Note: UWP app lifecycle cannot be tracked, so game state will show as "running"
/// but won't automatically reset when the game closes.
#[tauri::command]
pub async fn launch_xbox<R: Runtime>(app: AppHandle<R>, app_id: String) -> Result<(), String> {
    info!("launch_xbox: app_id={}", app_id);

    // Build the shell:AppsFolder URI
    let uri = format!("shell:AppsFolder\\{}", app_id);
    debug!("Launching via: {}", uri);

    // Use 'explorer' to open the shell URI - it returns immediately after launching
    Command::new("explorer").arg(&uri).spawn().map_err(|e| {
        error!("Failed to launch Xbox game: {}", e);
        format!("Failed to launch Xbox game: {e}")
    })?;

    // Emit game state (UWP app lifecycle cannot be tracked)
    let _ = app.emit("game-state-changed", GameStatePayload { running: true });
    info!("Xbox game launched (no process monitoring available for UWP apps)");

    Ok(())
}

#[cfg(not(windows))]
#[tauri::command]
pub async fn launch_xbox() -> Result<String, String> {
    Err("Xbox game launching is only supported on Windows".to_string())
}

/// Cleans up Xbox doorstop files from the game directory.
#[tauri::command]
pub async fn cleanup_xbox_files(game_dir: String) -> Result<(), String> {
    info!("cleanup_xbox_files: game_dir={}", game_dir);

    let game_dir = PathBuf::from(&game_dir);
    let dll_path = game_dir.join("winhttp.dll");
    let ini_path = game_dir.join("doorstop_config.ini");

    // Remove winhttp.dll if it exists
    if dll_path.exists() {
        debug!("Removing winhttp.dll from game directory");
        std::fs::remove_file(&dll_path).map_err(|e| {
            error!("Failed to remove winhttp.dll: {}", e);
            format!("Failed to remove winhttp.dll: {e}")
        })?;
    }

    // Remove doorstop_config.ini if it exists
    if ini_path.exists() {
        debug!("Removing doorstop_config.ini from game directory");
        std::fs::remove_file(&ini_path).map_err(|e| {
            error!("Failed to remove doorstop_config.ini: {}", e);
            format!("Failed to remove doorstop_config.ini: {e}")
        })?;
    }

    info!("Xbox cleanup complete");
    Ok(())
}

#[cfg(not(windows))]
#[tauri::command]
pub async fn cleanup_xbox_files() -> Result<String, String> {
    Err("Xbox game launching is only supported on Windows".to_string())
}
