use crate::backend::error::{AppError, AppResult};
use crate::backend::services::profile_service::ProfileEntry;
use crate::backend::state::game_runtime;
use log::{debug, info};
use serde::Deserialize;
use std::path::PathBuf;
use std::process::Command;

/// Serializes modded launches. Held across prep + spawn and then for the
/// per-launch settle delay afterwards, so a second launch fired in quick
/// succession queues behind the first instead of racing it. Among Us + BepInEx (IL2CPP)
/// write shared state in the profile dir (cache/interop) during startup, so a
/// second instance that starts mid-warm-up dies. Waiting until the first is up
/// — the manual "launch, wait for the console, launch again" workaround — is
/// what this automates.
static LAUNCH_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Per-profile cancellation counter. A queued launch records the value when it
/// starts waiting on [`LAUNCH_LOCK`]; if [`cancel_pending_launches`] has bumped
/// it by the time the lock is acquired, the launch aborts instead of spawning.
/// Lets the Stop button cancel launches still sitting in the settle queue.
static CANCEL_GENERATIONS: std::sync::LazyLock<
    std::sync::Mutex<std::collections::HashMap<String, u64>>,
> = std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::HashMap::new()));

fn cancel_generation(profile_id: &str) -> u64 {
    CANCEL_GENERATIONS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(profile_id)
        .copied()
        .unwrap_or(0)
}

/// Cancel any launches for `profile_id` still queued behind the launch lock.
/// They abort (without spawning) when they reach the front of the queue.
pub fn cancel_pending_launches(profile_id: &str) {
    *CANCEL_GENERATIONS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .entry(profile_id.to_string())
        .or_insert(0) += 1;
}

#[cfg(target_os = "linux")]
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
    /// Launch through the Steam client instead of running Proton ourselves.
    Steam,
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
    /// Seconds to hold the launch lock after spawning so a queued next launch
    /// waits for this instance to warm up. 0 disables the wait.
    pub settle_delay_secs: u64,
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

#[cfg(not(any(windows, target_os = "linux")))]
fn build_game_command(_game_exe: &str) -> AppResult<Command> {
    Err(AppError::Platform(
        "Launching the game is not supported on this platform".to_string(),
    ))
}

#[cfg(windows)]
fn set_dll_directory(path: &str) -> AppResult<()> {
    use windows::Win32::System::LibraryLoader::SetDllDirectoryW;
    use windows::core::PCWSTR;

    let wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe { SetDllDirectoryW(PCWSTR(wide.as_ptr())) }
        .map_err(|e| AppError::process(format!("SetDllDirectory failed: {e}")))
}

#[cfg(any(windows, target_os = "linux"))]
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
            // Steam launches branch to `steam -applaunch` before reaching here.
            LinuxRunner::Steam => unreachable!("Steam launches via steam -applaunch"),
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

fn attach_epic_launch_token(_cmd: &mut Command, platform: &str) -> AppResult<()> {
    // TODO: Re-implement Epic launch token attachment once the new
    // (link + paste-code) auth flow lands and we can persist EpicSessions
    // again. For now, Epic launches happen without `-AUTH_PASSWORD`.
    if platform == "epic" {
        debug!("Epic launch: skipping AUTH_PASSWORD (auth flow not yet implemented)");
    }
    Ok(())
}

fn launch_process(mut cmd: Command, profile_id: Option<String>) -> AppResult<()> {
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    let child = cmd
        .spawn()
        .map_err(|e| AppError::process(format!("Failed to launch game: {e}")))?;
    game_runtime::register_launched_process(child, profile_id)
}

/// Among Us' Steam app id.
#[cfg(target_os = "linux")]
const STEAM_APP_ID: &str = "945360";

/// Write a Doorstop `doorstop_config.ini` into the game dir. Used by the
/// Steam-launch path, where we can't pass `--doorstop-*` args on the command
/// line — Doorstop reads this file at startup instead. Paths are wine paths.
#[cfg(target_os = "linux")]
fn write_doorstop_ini(
    game_dir: &Path,
    target_assembly: &str,
    corlib_dir: &str,
    coreclr_path: &str,
) -> AppResult<()> {
    let ini = format!(
        "[General]\n\
         enabled = true\n\
         target_assembly = {target_assembly}\n\
         \n\
         [Il2Cpp]\n\
         coreclr_path = {coreclr_path}\n\
         corlib_dir = {corlib_dir}\n"
    );
    fs::write(game_dir.join("doorstop_config.ini"), ini)?;
    Ok(())
}

/// Write a disabled `doorstop_config.ini` so a vanilla Steam launch can't
/// inject mods even when the winhttp.dll proxy is still present (the Steam
/// launch option loads it unconditionally).
#[cfg(target_os = "linux")]
fn clear_doorstop_ini(game_dir: &Path) -> AppResult<()> {
    fs::write(
        game_dir.join("doorstop_config.ini"),
        "[General]\nenabled = false\n",
    )?;
    Ok(())
}

/// Spawn `steam -applaunch` and reap the short-lived invoker in the background.
/// Steam reparents the actual game, so we don't track this child — running
/// state is watched separately via [`game_runtime::register_steam_launch`].
#[cfg(target_os = "linux")]
fn spawn_steam(mut cmd: Command) -> AppResult<()> {
    use std::os::unix::process::CommandExt;
    cmd.process_group(0);
    let mut child = cmd
        .spawn()
        .map_err(|e| AppError::process(format!("Failed to launch via Steam: {e}")))?;
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

/// Modded launch handed to the Steam client instead of running Proton
/// ourselves. Steam owns the process, so Steamworks initializes (online play)
/// and the game runs inside the Steam Linux Runtime (audio). Doorstop config
/// goes through `doorstop_config.ini` since `steam -applaunch` can't forward
/// `--doorstop-*` args.
#[cfg(target_os = "linux")]
fn launch_modded_via_steam(args: &LaunchModdedArgs, game_dir: &Path) -> AppResult<()> {
    prepare_linux_winhttp_proxy(game_dir, &args.profile_path)?;
    write_doorstop_ini(
        game_dir,
        &to_wine_path(&args.bepinex_dll),
        &to_wine_path(&args.dotnet_dir),
        &to_wine_path(&args.coreclr_path),
    )?;

    let mut cmd = Command::new("steam");
    cmd.arg("-applaunch")
        .arg(STEAM_APP_ID)
        // Only reaches Proton if this call cold-starts Steam. If Steam is
        // already running, set this once in the game's Steam launch options:
        //   WINEDLLOVERRIDES="winhttp=n,b" %command%
        .env("WINEDLLOVERRIDES", "winhttp=n,b");
    spawn_steam(cmd)?;

    game_runtime::register_steam_launch(Some(args.profile_id.clone()))?;

    if args.settle_delay_secs > 0 {
        std::thread::sleep(std::time::Duration::from_secs(args.settle_delay_secs));
    }
    Ok(())
}

pub fn launch_modded(args: LaunchModdedArgs) -> AppResult<()> {
    info!("game_launch_modded: game_exe={}", args.game_exe);

    let game_dir = PathBuf::from(&args.game_exe)
        .parent()
        .ok_or_else(|| AppError::validation("Invalid game path"))?
        .to_path_buf();

    // Hold the launch lock from prep through spawn + settle (see LAUNCH_LOCK).
    let cancel_gen = cancel_generation(&args.profile_id);
    let _launch_guard = LAUNCH_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    if cancel_generation(&args.profile_id) != cancel_gen {
        info!("launch cancelled while queued: profile={}", args.profile_id);
        return Ok(());
    }

    // Steam runner: let the Steam client launch the game so online play
    // (Steamworks) and audio (Steam Linux Runtime) work, injecting via
    // doorstop_config.ini instead of command-line args.
    #[cfg(target_os = "linux")]
    if matches!(args.runner, LinuxRunner::Steam) {
        return launch_modded_via_steam(&args, &game_dir);
    }

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
    #[cfg(not(target_os = "linux"))]
    let bepinex_dll = args.bepinex_dll.clone();

    #[cfg(target_os = "linux")]
    let dotnet_dir = to_wine_path(&args.dotnet_dir);
    #[cfg(not(target_os = "linux"))]
    let dotnet_dir = args.dotnet_dir.clone();

    #[cfg(target_os = "linux")]
    let coreclr_path = to_wine_path(&args.coreclr_path);
    #[cfg(not(target_os = "linux"))]
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

    attach_epic_launch_token(&mut cmd, &args.platform)?;
    let result = launch_process(cmd, Some(args.profile_id));

    // Keep the lock for the settle window after a successful spawn so the next
    // queued launch waits for this instance to warm up the shared state.
    if result.is_ok() && args.settle_delay_secs > 0 {
        std::thread::sleep(std::time::Duration::from_secs(args.settle_delay_secs));
    }

    result
}

pub fn launch_vanilla(args: LaunchVanillaArgs) -> AppResult<()> {
    info!("game_launch_vanilla: game_exe={}", args.game_exe);

    let game_dir = PathBuf::from(&args.game_exe)
        .parent()
        .ok_or_else(|| AppError::validation("Invalid game path"))?
        .to_path_buf();

    // Steam runner: disable doorstop via the ini (clears any prior modded
    // config) and hand the launch to the Steam client.
    #[cfg(target_os = "linux")]
    if matches!(args.runner, LinuxRunner::Steam) {
        clear_doorstop_ini(&game_dir)?;
        let mut cmd = Command::new("steam");
        cmd.arg("-applaunch").arg(STEAM_APP_ID);
        spawn_steam(cmd)?;
        return game_runtime::register_steam_launch(None);
    }

    // Strip any modded-launch leftovers from the game directory so the
    // doorstop loader can't accidentally inject a previous profile's
    // BepInEx into a vanilla wine/proton session.
    #[cfg(target_os = "linux")]
    cleanup_linux_doorstop_files(&game_dir)?;

    let mut cmd = build_game_command(
        &args.game_exe,
        #[cfg(target_os = "linux")]
        &args.runner,
    )?;

    cmd.current_dir(&game_dir)
        .args(["--doorstop-enabled", "false"]);

    attach_epic_launch_token(&mut cmd, &args.platform)?;
    launch_process(cmd, None)
}

/// Self-contained vanilla launch: reads app settings, resolves the game
/// path and platform, builds the Linux runner if needed, and dispatches
/// [`launch_vanilla`]. Vanilla launches are profile-less by design.
pub fn launch_vanilla_from_settings() -> AppResult<()> {
    use crate::backend::services::core_service;

    let settings = core_service::get_settings()?;
    let game_path = settings.among_us_path.trim();
    if game_path.is_empty() {
        return Err(AppError::validation(
            "Among Us path is not set. Configure it in Settings.",
        ));
    }

    let game_exe = PathBuf::from(game_path).join(GAME_EXE_NAME);
    if !game_exe.exists() {
        return Err(AppError::validation(format!(
            "{GAME_EXE_NAME} not found at {}",
            game_exe.display()
        )));
    }

    let platform = match settings.game_platform {
        core_service::GamePlatform::Steam => "steam",
        core_service::GamePlatform::Epic => "epic",
        core_service::GamePlatform::Xbox => "xbox",
    }
    .to_string();

    #[cfg(target_os = "linux")]
    let runner = build_linux_runner_from_settings(&settings)?;

    launch_vanilla(LaunchVanillaArgs {
        game_exe: game_exe.to_string_lossy().to_string(),
        platform,
        #[cfg(target_os = "linux")]
        runner,
    })
}

#[cfg(target_os = "linux")]
fn build_linux_runner_from_settings(
    settings: &crate::backend::services::core_service::AppSettings,
) -> AppResult<LinuxRunner> {
    use crate::backend::services::core_service::LinuxRunnerKind;

    // Steam runs through the Steam client, so it needs no runner binary.
    if matches!(settings.linux_runner_kind, LinuxRunnerKind::Steam) {
        return Ok(LinuxRunner::Steam);
    }

    let binary = settings.linux_runner_binary.trim();
    if binary.is_empty() {
        return Err(AppError::validation(
            "Linux runner binary is required in Settings.",
        ));
    }
    Ok(match settings.linux_runner_kind {
        LinuxRunnerKind::Wine => LinuxRunner::Wine {
            binary: binary.to_string(),
            prefix: settings.linux_wine_prefix.clone(),
        },
        LinuxRunnerKind::Proton => LinuxRunner::Proton {
            binary: binary.to_string(),
            compat_data_path: settings.linux_proton_compat_data_path.clone(),
            steam_client_path: settings.linux_proton_steam_client_path.clone(),
            use_steam_run: settings.linux_proton_use_steam_run,
        },
        LinuxRunnerKind::Steam => unreachable!("handled above"),
    })
}

const GAME_EXE_NAME: &str = "Among Us.exe";

#[cfg(any(windows, target_os = "linux"))]
const CORECLR_FILE: &str = "coreclr.dll";
#[cfg(target_os = "macos")]
const CORECLR_FILE: &str = "libcoreclr.dylib";

/// Self-contained modded launch for the given profile. Reads app settings,
/// validates the game executable, BepInEx DLL, and dotnet runtime, then
/// dispatches [`launch_modded`].
pub fn launch_modded_for_profile(profile: ProfileEntry) -> AppResult<()> {
    use crate::backend::services::core_service;

    let settings = core_service::get_settings()?;
    let game_path = settings.among_us_path.trim();
    if game_path.is_empty() {
        return Err(AppError::validation(
            "Among Us path is not set. Configure it in Settings.",
        ));
    }

    let game_exe = PathBuf::from(game_path).join(GAME_EXE_NAME);
    if !game_exe.exists() {
        return Err(AppError::validation(format!(
            "{GAME_EXE_NAME} not found at {}",
            game_exe.display()
        )));
    }

    let profile_path = PathBuf::from(&profile.path);
    let bepinex_dll = profile_path
        .join("BepInEx")
        .join("core")
        .join("BepInEx.Unity.IL2CPP.dll");
    if !bepinex_dll.exists() {
        return Err(AppError::validation(
            "BepInEx DLL not found. Install BepInEx for this profile first.",
        ));
    }
    let dotnet_dir = profile_path.join("dotnet");
    let coreclr_path = dotnet_dir.join(CORECLR_FILE);
    if !coreclr_path.exists() {
        return Err(AppError::validation(format!(
            "dotnet runtime not found at {}",
            coreclr_path.display()
        )));
    }

    let platform = match settings.game_platform {
        core_service::GamePlatform::Steam => "steam",
        core_service::GamePlatform::Epic => "epic",
        core_service::GamePlatform::Xbox => "xbox",
    }
    .to_string();

    #[cfg(target_os = "linux")]
    let runner = build_linux_runner_from_settings(&settings)?;

    // Only wait between launches when multiple instances are allowed — that's
    // the only time a second launch can be queued behind this one.
    let settle_delay_secs = if settings.allow_multi_instance_launch {
        settings.multi_instance_launch_delay_secs
    } else {
        0
    };

    launch_modded(LaunchModdedArgs {
        game_exe: game_exe.to_string_lossy().to_string(),
        profile_id: profile.id.clone(),
        #[cfg(any(windows, target_os = "linux"))]
        profile_path: profile.path.clone(),
        bepinex_dll: bepinex_dll.to_string_lossy().to_string(),
        dotnet_dir: dotnet_dir.to_string_lossy().to_string(),
        coreclr_path: coreclr_path.to_string_lossy().to_string(),
        platform,
        settle_delay_secs,
        #[cfg(target_os = "linux")]
        runner,
    })
}
