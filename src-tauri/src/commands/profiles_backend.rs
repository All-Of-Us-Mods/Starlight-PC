use std::path::Path;

/// Windows-only: Set DLL directory for Doorstop.
/// Required because Tauri Shell plugin doesn't expose this environment tweak.
#[tauri::command]
#[cfg(windows)]
pub fn set_dll_directory(profile_path: String) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;

    let wide: Vec<u16> = Path::new(&profile_path)
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let success = unsafe { SetDllDirectoryW(wide.as_ptr()) };

    if success == 0 {
        Err(format!(
            "Failed to set DLL directory: {}",
            std::io::Error::last_os_error()
        ))
    } else {
        Ok(())
    }
}

#[cfg(windows)]
#[link(name = "Kernel32")]
extern "system" {
    fn SetDllDirectoryW(lp_path_name: *const u16) -> i32;
}

/// Check if Among Us is already running.
#[tauri::command]
pub fn check_among_us_running() -> Result<bool, String> {
    use std::ffi::OsStr;
    use sysinfo::System;

    let mut sys = System::new_all();
    sys.refresh_all();

    let is_running = sys
        .processes_by_exact_name(OsStr::new("Among Us"))
        .next()
        .is_some();

    Ok(is_running)
}

#[cfg(not(windows))]
#[tauri::command]
pub fn set_dll_directory(_profile_path: String) -> Result<(), String> {
    Ok(())
}
