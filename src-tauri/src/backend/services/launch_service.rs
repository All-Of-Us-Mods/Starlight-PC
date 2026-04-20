use crate::backend::error::{AppError, AppResult};
use crate::backend::services::epic_auth_service::{EpicAuthService, load_session};
use crate::backend::state::game_runtime;
use log::{debug, info, warn};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tauri::{AppHandle, Runtime};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum LinuxRunner {
    Wine {
        binary: String,
        prefix: String,
    },
    Proton {
        binary: String,
        #[serde(rename = "compatDataPath")]
        compat_data_path: String,
        #[serde(rename = "steamClientPath")]
        steam_client_path: String,
        #[serde(rename = "useSteamRun")]
        use_steam_run: bool,
    },
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchModdedArgs {
    pub game_exe: String,
    pub profile_id: String,
    #[cfg(any(windows, target_os = "linux"))]
    pub profile_path: String,
    pub bepinex_dll: String,
    pub dotnet_dir: String,
    pub coreclr_path: String,
    pub platform: String,
    #[cfg(target_os = "linux")]
    pub runner: LinuxRunner,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchVanillaArgs {
    pub game_exe: String,
    pub platform: String,
    #[cfg(target_os = "linux")]
    pub runner: LinuxRunner,
}

#[cfg(windows)]
fn set_dll_directory(path: &str) -> AppResult<()> {
    use windows::Win32::System::LibraryLoader::SetDllDirectoryW;
    use windows::core::PCWSTR;

    let wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe { SetDllDirectoryW(PCWSTR(wide.as_ptr())) }
        .map_err(|e| AppError::process(format!("SetDllDirectory failed: {e}")))
}

fn build_game_command(
    game_exe: &str,
    #[cfg(target_os = "linux")] runner: &LinuxRunner,
) -> AppResult<Command> {
    #[cfg(windows)]
    {
        Ok(Command::new(game_exe))
    }

    #[cfg(target_os = "linux")]
    {
        const STEAM_RUN: &str = "steam-run";

        let cmd = match runner {
            LinuxRunner::Wine { binary, prefix } => {
                let mut cmd = Command::new(binary);
                cmd.env("WINEPREFIX", prefix).arg(game_exe);
                cmd
            }
            LinuxRunner::Proton {
                binary,
                compat_data_path,
                steam_client_path,
                use_steam_run,
            } => {
                let mut cmd = if *use_steam_run {
                    let mut steam = Command::new(STEAM_RUN);
                    steam.arg(binary);
                    steam
                } else {
                    Command::new(binary)
                };

                cmd.env("STEAM_COMPAT_DATA_PATH", compat_data_path)
                    .env("STEAM_COMPAT_CLIENT_INSTALL_PATH", steam_client_path)
                    .env("WINEPREFIX", format!("{compat_data_path}/pfx"))
                    .arg("waitforexitandrun")
                    .arg(game_exe);
                cmd
            }
        };

        Ok(cmd)
    }
}

#[cfg(target_os = "linux")]
fn to_wine_path(path: &str) -> String {
    if path.starts_with('/') {
        format!("Z:{}", path.replace('/', "\\"))
    } else {
        path.to_string()
    }
}

#[cfg(target_os = "linux")]
fn prepare_linux_winhttp_proxy(game_dir: &Path, profile_path: &str) -> AppResult<()> {
    let profile_dir = PathBuf::from(profile_path);
    let src_dll = profile_dir.join("winhttp.dll");
    let dst_dll = game_dir.join("winhttp.dll");
    let dst_ini = game_dir.join("doorstop_config.ini");

    if !src_dll.exists() {
        return Err(AppError::validation(
            "winhttp.dll not found in profile. Please wait for BepInEx installation to complete.",
        ));
    }

    fs::copy(&src_dll, &dst_dll)?;

    if dst_ini.exists() {
        fs::remove_file(dst_ini)?;
    }

    Ok(())
}

#[cfg(target_os = "linux")]
fn cleanup_linux_doorstop_files(game_dir: &Path) -> AppResult<()> {
    let dll_path = game_dir.join("winhttp.dll");
    let ini_path = game_dir.join("doorstop_config.ini");

    if dll_path.exists() {
        fs::remove_file(dll_path)?;
    }
    if ini_path.exists() {
        fs::remove_file(ini_path)?;
    }
    Ok(())
}

async fn attach_epic_launch_token(cmd: &mut Command, platform: &str) -> AppResult<()> {
    if platform != "epic" {
        return Ok(());
    }

    let Some(session) = load_session() else {
        return Ok(());
    };

    info!("Epic session found, requesting game token");
    let api = EpicAuthService::new()?;
    match api.get_game_token(&session).await {
        Ok(launch_token) => {
            debug!("Epic game token obtained successfully");
            cmd.arg(format!("-AUTH_PASSWORD={}", launch_token));
        }
        Err(e) => warn!("Failed to get Epic game token: {}", e),
    }

    Ok(())
}

fn launch_process<R: Runtime>(
    app: AppHandle<R>,
    mut cmd: Command,
    profile_id: Option<String>,
) -> AppResult<()> {
    let child = cmd
        .spawn()
        .map_err(|e| AppError::process(format!("Failed to launch game: {e}")))?;
    game_runtime::register_launched_process(app, child, profile_id)
}

pub async fn launch_modded<R: Runtime>(app: AppHandle<R>, args: LaunchModdedArgs) -> AppResult<()> {
    info!("game_launch_modded: game_exe={}", args.game_exe);

    let game_dir = PathBuf::from(&args.game_exe)
        .parent()
        .ok_or_else(|| AppError::validation("Invalid game path"))?
        .to_path_buf();

    #[cfg(windows)]
    set_dll_directory(&args.profile_path)?;

    #[cfg(target_os = "linux")]
    prepare_linux_winhttp_proxy(&game_dir, &args.profile_path)?;

    let mut cmd = build_game_command(
        &args.game_exe,
        #[cfg(target_os = "linux")]
        &args.runner,
    )?;

    #[cfg(target_os = "linux")]
    let bepinex_dll = to_wine_path(&args.bepinex_dll);
    #[cfg(windows)]
    let bepinex_dll = args.bepinex_dll.clone();

    #[cfg(target_os = "linux")]
    let dotnet_dir = to_wine_path(&args.dotnet_dir);
    #[cfg(windows)]
    let dotnet_dir = args.dotnet_dir.clone();

    #[cfg(target_os = "linux")]
    let coreclr_path = to_wine_path(&args.coreclr_path);
    #[cfg(windows)]
    let coreclr_path = args.coreclr_path.clone();

    cmd.current_dir(&game_dir)
        .args(["--doorstop-enabled", "true"])
        .args(["--doorstop-target-assembly", &bepinex_dll])
        .args(["--doorstop-clr-corlib-dir", &dotnet_dir])
        .args(["--doorstop-clr-runtime-coreclr-path", &coreclr_path]);

    #[cfg(target_os = "linux")]
    {
        cmd.env("WINEDLLOVERRIDES", "winhttp=n,b");
    }

    attach_epic_launch_token(&mut cmd, &args.platform).await?;
    launch_process(app, cmd, Some(args.profile_id))
}

pub async fn launch_vanilla<R: Runtime>(
    app: AppHandle<R>,
    args: LaunchVanillaArgs,
) -> AppResult<()> {
    #[cfg(target_os = "linux")]
    {
        let game_dir = PathBuf::from(&args.game_exe)
            .parent()
            .ok_or_else(|| AppError::validation("Invalid game path"))?
            .to_path_buf();
        cleanup_linux_doorstop_files(&game_dir)?;
    }

    let mut cmd = build_game_command(
        &args.game_exe,
        #[cfg(target_os = "linux")]
        &args.runner,
    )?;

    attach_epic_launch_token(&mut cmd, &args.platform).await?;
    launch_process(app, cmd, None)
}
