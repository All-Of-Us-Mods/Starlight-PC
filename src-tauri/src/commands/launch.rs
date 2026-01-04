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

    if let Some(session) = epic_api::load_session() {
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
pub async fn launch_vanilla<R: Runtime>(app: AppHandle<R>, game_exe: String) -> Result<(), String> {
    info!("launch_vanilla: game_exe={}", game_exe);
    let mut cmd = Command::new(&game_exe);

    if let Some(session) = epic_api::load_session() {
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
