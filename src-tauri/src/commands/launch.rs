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

    let wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe { SetDllDirectoryW(PCWSTR(wide.as_ptr())) }
        .map_err(|e| format!("SetDllDirectory failed: {e}"))
}

fn launch<R: Runtime>(app: AppHandle<R>, mut cmd: Command) -> Result<(), String> {
    {
        let mut guard = GAME_PROCESS.lock().unwrap();

        if guard
            .as_mut()
            .is_some_and(|c| c.try_wait().ok().flatten().is_none())
        {
            return Err("Game is already running".into());
        }

        let child = cmd
            .spawn()
            .map_err(|e| format!("Failed to launch game: {e}"))?;
        *guard = Some(child);
    }

    std::thread::spawn(move || {
        let _ = app.emit("game-state-changed", GameStatePayload { running: true });

        loop {
            std::thread::sleep(Duration::from_millis(500));

            let Ok(mut guard) = GAME_PROCESS.lock() else {
                break;
            };

            match guard.as_mut().and_then(|c| c.try_wait().ok()) {
                Some(Some(_)) | None => {
                    *guard = None;
                    break;
                }
                Some(None) => {}
            }
        }

        let _ = app.emit("game-state-changed", GameStatePayload { running: false });
    });

    Ok(())
}

#[tauri::command]
pub fn launch_modded<R: Runtime>(
    app: AppHandle<R>,
    game_exe: String,
    _profile_path: String,
    bepinex_dll: String,
    dotnet_dir: String,
    coreclr_path: String,
) -> Result<(), String> {
    let game_dir = PathBuf::from(&game_exe);
    let game_dir = game_dir.parent().ok_or("Invalid game path")?;

    #[cfg(windows)]
    set_dll_directory(&_profile_path)?;

    let mut cmd = Command::new(&game_exe);
    cmd.current_dir(game_dir)
        .args(["--doorstop-enabled", "true"])
        .args(["--doorstop-target-assembly", &bepinex_dll])
        .args(["--doorstop-clr-corlib-dir", &dotnet_dir])
        .args(["--doorstop-clr-runtime-coreclr-path", &coreclr_path]);

    launch(app, cmd)
}

#[tauri::command]
pub fn launch_vanilla<R: Runtime>(app: AppHandle<R>, game_exe: String) -> Result<(), String> {
    launch(app, Command::new(&game_exe))
}
