use log::info;
use std::path::{Path, PathBuf};

#[cfg(target_os = "windows")]
use {std::ffi::OsStr, sysinfo::System, winreg::enums::*, winreg::RegKey};

const AMONG_US_EXE: &str = "Among Us.exe";

fn verify_among_us_directory(path: &Path) -> bool {
    path.is_dir() && path.join(AMONG_US_EXE).is_file()
}

#[cfg(target_os = "windows")]
fn find_among_us_from_processes() -> Option<PathBuf> {
    let system = System::new_all();

    for name in [AMONG_US_EXE, "Among Us"] {
        let process = system.processes_by_exact_name(OsStr::new(name)).next()?;
        let exe_path = process.exe().filter(|p| !p.as_os_str().is_empty())?;
        let directory = exe_path.parent()?;

        if verify_among_us_directory(directory) {
            info!("Found Among Us via process: {}", directory.display());
            return Some(directory.to_path_buf());
        }
    }

    None
}

#[cfg(target_os = "windows")]
fn parse_registry_icon_value(raw_value: &str) -> Option<PathBuf> {
    let path = raw_value
        .split(',')
        .next()?
        .trim()
        .trim_matches(|c| c == '"' || c == '\'');

    if path.is_empty() {
        return None;
    }

    PathBuf::from(path).parent().map(|p| p.to_path_buf())
}

#[cfg(target_os = "windows")]
fn find_among_us_from_registry() -> Option<PathBuf> {
    let hkcr = RegKey::predef(HKEY_CLASSES_ROOT);

    for key_name in ["AmongUs", "amongus"] {
        let icon_key = hkcr.open_subkey(key_name).ok()?.open_subkey("DefaultIcon").ok()?;
        let raw_value: String = icon_key.get_value("").ok()?;
        let directory = parse_registry_icon_value(&raw_value)?;

        if verify_among_us_directory(&directory) {
            info!("Found Among Us via registry: {}", directory.display());
            return Some(directory);
        }
    }

    None
}

#[cfg(target_os = "windows")]
pub fn get_among_us_paths() -> Vec<PathBuf> {
    find_among_us_from_processes()
        .or_else(find_among_us_from_registry)
        .map(|path| vec![path])
        .unwrap_or_else(|| {
            info!("Among Us installation not detected");
            Vec::new()
        })
}

#[cfg(target_os = "linux")]
pub fn get_among_us_paths() -> Vec<PathBuf> {
    home::home_dir()
        .map(|mut path| {
            path.push(".local/share/Steam/steamapps/common/Among Us");
            path
        })
        .filter(|path| path.is_dir())
        .into_iter()
        .collect()
}

pub fn is_among_us_running() -> bool {
    #[cfg(target_os = "windows")]
    {
        let system = System::new_all();
        [AMONG_US_EXE, "Among Us"]
            .iter()
            .any(|name| system.processes_by_exact_name(OsStr::new(name)).next().is_some())
    }

    #[cfg(target_family = "unix")]
    {
        use libproc::{proc_pid::name, processes::{self, ProcFilter}};

        processes::pids_by_type(ProcFilter::All)
            .ok()
            .map(|pids| {
                pids.iter().any(|&pid| {
                    name(pid as i32)
                        .ok()
                        .map_or(false, |n| n.to_lowercase().contains("among us"))
                })
            })
            .unwrap_or(false)
    }
}
