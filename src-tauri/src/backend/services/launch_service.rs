use crate::backend::error::{AppError, AppResult};
use crate::backend::services::epic_auth_service::{EpicAuthService, load_session};
use crate::backend::state::game_runtime;
use log::{debug, info, warn};
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tauri::{AppHandle, Runtime};

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
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchVanillaArgs {
    pub game_exe: String,
    pub platform: String,
}

#[cfg(windows)]
fn set_dll_directory(path: &str) -> AppResult<()> {
    use windows::Win32::System::LibraryLoader::SetDllDirectoryW;
    use windows::core::PCWSTR;

    let wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe { SetDllDirectoryW(PCWSTR(wide.as_ptr())) }
        .map_err(|e| AppError::process(format!("SetDllDirectory failed: {e}")))
}

fn build_game_command(game_exe: &str) -> AppResult<Command> {
    #[cfg(windows)]
    {
        Ok(Command::new(game_exe))
    }

    #[cfg(target_os = "linux")]
    {
        const STEAM_RUN: &str = "steam-run";
        const PROTON_BINARY: &str =
            "/home/yanpla/.local/share/Steam/steamapps/common/Proton - Experimental/proton";
        const STEAM_COMPAT_DATA_PATH: &str = "/mnt/games/SteamLibrary/steamapps/compatdata/945360";
        const STEAM_CLIENT_PATH: &str = "/home/yanpla/.local/share/Steam";

        let wine_prefix = format!("{STEAM_COMPAT_DATA_PATH}/pfx");

        let mut cmd = Command::new(STEAM_RUN);
        cmd.env("STEAM_COMPAT_DATA_PATH", STEAM_COMPAT_DATA_PATH)
            .env("STEAM_COMPAT_CLIENT_INSTALL_PATH", STEAM_CLIENT_PATH)
            .env("WINEPREFIX", wine_prefix)
            .arg(PROTON_BINARY)
            .arg("waitforexitandrun")
            .arg(game_exe);

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
fn prepare_linux_winhttp_proxy(game_dir: &PathBuf, profile_path: &str) -> AppResult<()> {
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
fn cleanup_linux_doorstop_files(game_dir: &PathBuf) -> AppResult<()> {
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

    let mut cmd = build_game_command(&args.game_exe)?;

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

    let mut cmd = build_game_command(&args.game_exe)?;

    attach_epic_launch_token(&mut cmd, &args.platform).await?;
    launch_process(app, cmd, None)
}
