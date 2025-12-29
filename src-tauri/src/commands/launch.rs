use once_cell::sync::Lazy;
use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Runtime};

#[cfg(windows)]
#[link(name = "Kernel32")]
extern "system" {
    fn SetDllDirectoryW(lp_path_name: *const u16) -> i32;
}

static GAME_PROCESS: Lazy<Mutex<Option<Child>>> = Lazy::new(|| Mutex::new(None));

#[derive(Clone, serde::Serialize)]
pub struct GameStatePayload {
    pub running: bool,
}

fn start_process_monitor<R: Runtime>(app: AppHandle<R>) {
    std::thread::spawn(move || {
        // Emit that game started
        let _ = app.emit("game-state-changed", GameStatePayload { running: true });

        loop {
            std::thread::sleep(Duration::from_millis(500));

            let mut guard = GAME_PROCESS.lock().map_err(|_| "Failed to acquire process lock")?;
            if let Some(ref mut child) = *guard {
                match child.try_wait() {
                    Ok(Some(_status)) => {
                        // Process exited
                        *guard = None;
                        drop(guard);
                        let _ = app.emit("game-state-changed", GameStatePayload { running: false });
                        break;
                    }
                    Ok(None) => {
                        // Still running
                    }
                    Err(_) => {
                        // Error checking status, assume dead
                        *guard = None;
                        drop(guard);
                        let _ = app.emit("game-state-changed", GameStatePayload { running: false });
                        break;
                    }
                }
            } else {
                break;
            }
        }
    });
}

#[tauri::command]
pub fn launch_modded<R: Runtime>(
    app: AppHandle<R>,
    game_exe: String,
    profile_path: String,
    bepinex_dll: String,
    dotnet_dir: String,
    coreclr_path: String,
) -> Result<(), String> {
    let game_path = PathBuf::from(&game_exe);
    let game_dir = game_path.parent().ok_or("Invalid game path")?;

    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        let profile_dir = PathBuf::from(&profile_path);
        let wide: Vec<u16> = profile_dir
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        unsafe {
            SetDllDirectoryW(wide.as_ptr());
        }
    }
    #[cfg(not(windows))]
    let _ = profile_path;

    let child = {
        let mut guard = GAME_PROCESS.lock().unwrap();
        if let Some(ref mut child) = *guard {
            if child.try_wait().ok().flatten().is_none() {
                return Err("Game is already running".into());
            }
        }

        Command::new(&game_exe)
            .current_dir(game_dir)
            .arg("--doorstop-enabled")
            .arg("true")
            .arg("--doorstop-target-assembly")
            .arg(&bepinex_dll)
            .arg("--doorstop-clr-corlib-dir")
            .arg(&dotnet_dir)
            .arg("--doorstop-clr-runtime-coreclr-path")
            .arg(&coreclr_path)
            .spawn()
            .map_err(|e| format!("Failed to spawn Among Us: {e}"))?
    };

    // Store process first, then start monitor
    *GAME_PROCESS.lock().unwrap() = Some(child);
    start_process_monitor(app);

    Ok(())
}

#[tauri::command]
pub fn launch_vanilla<R: Runtime>(app: AppHandle<R>, game_exe: String) -> Result<(), String> {
    let child = {
        let mut guard = GAME_PROCESS.lock().unwrap();
        if let Some(ref mut child) = *guard {
            if child.try_wait().ok().flatten().is_none() {
                return Err("Game is already running".into());
            }
        }

        Command::new(&game_exe)
            .spawn()
            .map_err(|e| format!("Failed to launch Among Us: {e}"))?
    };

    *GAME_PROCESS.lock().unwrap() = Some(child);
    start_process_monitor(app);

    Ok(())
}
