use log::{error, info};
use std::path::{Path, PathBuf};
#[cfg(target_os = "windows")]
use std::ffi::OsStr;
#[cfg(target_os = "windows")]
use sysinfo::System;
#[cfg(target_os = "windows")]
use winreg::enums::*;
#[cfg(target_os = "windows")]
use winreg::RegKey;

#[cfg(target_os = "windows")]
const AMONG_US_EXE: &str = "Among Us.exe";

#[cfg(target_os = "windows")]
fn verify_among_us_directory(path: &Path) -> bool {
    path.is_dir() && path.join(AMONG_US_EXE).is_file()
}

#[cfg(target_os = "windows")]
fn find_among_us_from_processes() -> Option<PathBuf> {
    let system = System::new_all();

    for name in [OsStr::new(AMONG_US_EXE), OsStr::new("Among Us")] {
        if let Some(process) = system.processes_by_exact_name(name).next() {
            let exe_path = match process.exe() {
                Some(path) if !path.as_os_str().is_empty() => path,
                _ => continue,
            };

            if let Some(directory) = exe_path.parent() {
                if verify_among_us_directory(directory) {
                    info!(
                        "Resolved Among Us directory from running process: {}",
                        directory.display()
                    );
                    return Some(directory.to_path_buf());
                } else {
                    error!(
                        "Process reported Among Us directory '{}' but verification failed.",
                        directory.display()
                    );
                }
            }
        }
    }

    None
}

#[cfg(target_os = "windows")]
fn parse_registry_icon_value(raw_value: &str) -> Option<PathBuf> {
    let without_args = raw_value.split(',').next()?.trim();
    let unquoted = without_args.trim_matches(|c| c == '"' || c == '\'');

    if unquoted.is_empty() {
        return None;
    }

    let exe_path = PathBuf::from(unquoted);
    exe_path.parent().map(|parent| parent.to_path_buf())
}

#[cfg(target_os = "windows")]
fn find_among_us_from_registry() -> Option<PathBuf> {
    let hkcr = RegKey::predef(HKEY_CLASSES_ROOT);

    for key_name in ["AmongUs", "amongus"] {
        let among_us_key = match hkcr.open_subkey(key_name) {
            Ok(key) => key,
            Err(_) => continue,
        };

        let default_icon_key = match among_us_key.open_subkey("DefaultIcon") {
            Ok(key) => key,
            Err(_) => continue,
        };

        let raw_value: String = match default_icon_key.get_value("") {
            Ok(value) => value,
            Err(_) => continue,
        };

        if let Some(directory) = parse_registry_icon_value(&raw_value) {
            if verify_among_us_directory(&directory) {
                info!(
                    "Resolved Among Us directory from registry: {}",
                    directory.display()
                );
                return Some(directory);
            } else {
                error!(
                    "Registry reported Among Us directory '{}' but verification failed.",
                    directory.display()
                );
            }
        }
    }

    None
}

#[cfg(target_os = "windows")]
pub fn get_among_us_paths() -> Vec<PathBuf> {
    if let Some(path) = find_among_us_from_processes() {
        return vec![path];
    }

    if let Some(path) = find_among_us_from_registry() {
        return vec![path];
    }

    info!("Among Us installation could not be detected via processes or registry.");
    Vec::new()
}

// #[cfg(target_os = "macos")]
// pub fn get_among_us_paths() -> Vec<PathBuf> {
//     let mut paths: Vec<PathBuf> = vec![];

//     // Prefer custom DB path first
//     if let Ok(db) = Database::new() {
//         if let Ok(Some(custom_path)) = db.get_installation_path() {
//             let p = PathBuf::from(&custom_path);
//             if p.exists() {
//                 paths.push(p);
//             }
//         }
//     }
//     match home::home_dir() {
//         Some(path) => {
//             let mut path = path;
//             path.push("Library/Application Support/Steam/steamapps/common/Among Us");
//             paths.push(path);
//         }
//         None => error!("Impossible to get your home dir!"),
//     }
//     remove_unexisting_paths(&mut paths);
//     dedup_paths_case_insensitive(&mut paths);
//     paths
// }

#[cfg(target_os = "linux")]
pub fn get_among_us_paths() -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = vec![];

    match home::home_dir() {
        Some(path) => {
            let mut path = path;
            path.push(".local/share/Steam/steamapps/common/Among Us");
            if path.is_dir() {
                paths.push(path);
            } else {
                error!("Among Us directory '{}' not found.", path.display());
            }
        }
        None => error!("Impossible to get your home dir!"),
    }

    paths
}

pub fn is_among_us_running() -> bool {
    #[cfg(target_os = "windows")]
    {
        let system = System::new_all();
        system
            .processes_by_exact_name(OsStr::new(AMONG_US_EXE))
            .next()
            .is_some()
            || system
                .processes_by_exact_name(OsStr::new("Among Us"))
                .next()
                .is_some()
    }

    #[cfg(target_family = "unix")]
    {
        use libproc::proc_pid::name;
        use libproc::processes;

        if let Ok(pids) = processes::pids_by_type(processes::ProcFilter::All) {
            for pid in pids {
                if let Ok(name) = name(pid as i32) {
                    if (name.to_lowercase().contains("among us")) {
                        return true;
                    }
                }
            }
        }
        false
    }
}
