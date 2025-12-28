use std::path::PathBuf;
use std::process::Command;

#[cfg(windows)]
#[link(name = "Kernel32")]
extern "system" {
    fn SetDllDirectoryW(lp_path_name: *const u16) -> i32;
}

#[tauri::command]
pub fn launch_modded(
    game_exe: String,
    profile_path: String,
    bepinex_dll: String,
    dotnet_dir: String,
    coreclr_path: String,
) -> Result<(), String> {
    let game_path = PathBuf::from(&game_exe);
    let game_dir = game_path.parent().ok_or("Invalid game path")?;
    #[cfg(not(windows))]
    let _ = profile_path;

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

    let mut cmd = Command::new(&game_exe);
    cmd.current_dir(game_dir)
        .arg("--doorstop-enabled")
        .arg("true")
        .arg("--doorstop-target-assembly")
        .arg(&bepinex_dll)
        .arg("--doorstop-clr-corlib-dir")
        .arg(&dotnet_dir)
        .arg("--doorstop-clr-runtime-coreclr-path")
        .arg(&coreclr_path);

    cmd.spawn()
        .map_err(|e| format!("Failed to spawn Among Us: {e}"))?;

    Ok(())
}

/// Launch Among Us in vanilla mode (without mods)
#[tauri::command]
pub fn launch_vanilla(game_exe: String) -> Result<(), String> {
    let mut cmd = Command::new(&game_exe);

    cmd.spawn()
        .map_err(|e| format!("Failed to launch Among Us: {e}"))?;

    Ok(())
}
