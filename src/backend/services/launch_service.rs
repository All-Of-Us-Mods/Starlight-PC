use crate::backend::api::LobbyMod;
use crate::backend::error::{AppError, AppResult};
use crate::backend::services::core_service::{self, GamePlatform};
use crate::backend::services::installation_service::GameSetup;
use crate::backend::services::mod_install_service::{self, InstallModInput};
use crate::backend::services::profile_service::{self, ProfileEntry};
#[cfg(windows)]
use crate::backend::services::xbox_service;
use crate::backend::services::{profile_instance_service, region_service};
use crate::backend::state::game_runtime::{self, LaunchInstance, PendingLaunch};
use log::{debug, info, warn};
use rust_i18n::t;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Held from prep through spawn so concurrent launches can't claim the same
/// instance slot or race over the shared game directory.
static LAUNCH_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(target_os = "linux")]
#[derive(Debug, Clone)]
pub enum LinuxRunner {
    Wine {
        binary: String,
        prefix: String,
    },
    Proton {
        binary: String,
        compat_data_path: String,
        steam_client_path: String,
        use_steam_run: bool,
    },
    Steam {
        compat_data_path: String,
    },
}

pub struct LaunchModdedArgs {
    pub game_exe: String,
    pub profile_id: String,
    pub profile_path: String,
    pub bepinex_dll: String,
    pub dotnet_dir: String,
    pub coreclr_path: String,
    pub platform: GamePlatform,
    pub allow_instance_copy: bool,
    #[cfg(target_os = "linux")]
    pub runner: LinuxRunner,
}

pub struct LaunchVanillaArgs {
    pub game_exe: String,
    pub platform: GamePlatform,
    #[cfg(target_os = "linux")]
    pub runner: LinuxRunner,
}

#[cfg(not(any(windows, target_os = "linux")))]
fn build_game_command(_game_exe: &str) -> AppResult<Command> {
    Err(AppError::platform(
        "Launching the game is not supported on this platform",
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

                // Not `waitforexitandrun`: it waits for the running instance's
                // wineserver to exit first.
                cmd.env("STEAM_COMPAT_DATA_PATH", compat_data_path)
                    .env("STEAM_COMPAT_CLIENT_INSTALL_PATH", steam_client_path)
                    .env("WINEPREFIX", format!("{compat_data_path}/pfx"))
                    .arg("run")
                    .arg(game_exe);
                cmd
            }
            // The first Steam instance goes through `steam -applaunch`.
            LinuxRunner::Steam { .. } => steam_alongside_command()?,
        };

        Ok(cmd)
    }
}

/// Steam won't launch an app twice, so further instances rerun Steam's own
/// Proton process (same container and environment, so online play and audio
/// keep working) with `run` in place of `waitforexitandrun`.
#[cfg(target_os = "linux")]
fn steam_alongside_command() -> AppResult<Command> {
    let proton = game_runtime::steam_proton().ok_or_else(|| {
        AppError::process("The Among Us instance Steam started has already exited.")
    })?;
    let mut argv = proton.argv.into_iter().map(|arg| {
        if arg == "waitforexitandrun" {
            "run".into()
        } else {
            arg
        }
    });
    let program = argv
        .next()
        .ok_or_else(|| AppError::process("Steam's Proton command line is empty."))?;
    let mut env = proton.env;
    match env.iter_mut().find(|(key, _)| key == "WINEDLLOVERRIDES") {
        Some((_, overrides)) => overrides.push(";winhttp=n,b"),
        None => env.push(("WINEDLLOVERRIDES".into(), "winhttp=n,b".into())),
    }

    let mut cmd = if proton.sandboxed {
        // nsenter keeps our environment (Steam's preloads an overlay our
        // libraries can't satisfy); `env -i` passes Steam's on inside.
        let mut cmd = Command::new("nsenter");
        cmd.arg(format!("--target={}", proton.pid));
        if proton.own_user_namespace {
            cmd.args(["--user", "--preserve-credentials"]);
        }
        cmd.args(["--mount", "--root", "--wd", "--", "env", "-i"]);
        for (key, value) in env {
            let mut pair = key;
            pair.push("=");
            pair.push(value);
            cmd.arg(pair);
        }
        cmd.arg(program);
        cmd
    } else {
        let mut cmd = Command::new(program);
        cmd.env_clear().envs(env);
        cmd
    };
    cmd.args(argv);
    Ok(cmd)
}

/// Whether Steam is running the game. Waits out a Steam launch that is still
/// starting: launching again in that window would be ignored by Steam.
#[cfg(target_os = "linux")]
fn steam_game_running(cancelled: impl Fn() -> bool) -> bool {
    let started = std::time::Instant::now();
    loop {
        if game_runtime::steam_proton().is_some() {
            return true;
        }
        if !game_runtime::steam_launch_pending()
            || cancelled()
            || started.elapsed() > std::time::Duration::from_secs(120)
        {
            return false;
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}

/// A host path as the game sees it: under wine the Unix root is drive `Z:`.
fn game_path(path: &str) -> String {
    if cfg!(target_os = "linux") && path.starts_with('/') {
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

    if !src_dll.exists() {
        return Err(AppError::validation(
            "winhttp.dll not found in profile. Please wait for BepInEx installation to complete.",
        ));
    }

    // A running instance has this DLL mapped; never truncate it in place.
    if fs::read(&dst_dll).ok() != Some(fs::read(&src_dll)?) {
        let temporary_dll = game_dir.join("winhttp.dll.starlight-tmp");
        fs::copy(&src_dll, &temporary_dll)?;
        fs::rename(&temporary_dll, &dst_dll)?;
    }

    Ok(())
}

/// Borrows auth arguments from an Epic-launcher-started instance so our copy
/// authenticates like a normal Epic launch.
#[cfg(windows)]
fn attach_epic_launch_args(cmd: &mut Command, platform: GamePlatform) -> AppResult<()> {
    use std::os::windows::process::CommandExt as _;

    if platform != GamePlatform::Epic {
        return Ok(());
    }

    let args = crate::backend::services::epic_launch_service::acquire_launch_args()?;
    cmd.raw_arg(args);
    Ok(())
}

#[cfg(not(windows))]
fn attach_epic_launch_args(_cmd: &mut Command, _platform: GamePlatform) -> AppResult<()> {
    Ok(())
}

fn launch_process(
    mut cmd: Command,
    pending: Option<&PendingLaunch>,
    instance: LaunchInstance,
) -> AppResult<()> {
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    let id = game_runtime::next_instance_id();
    #[cfg(target_os = "linux")]
    cmd.arg(game_runtime::instance_arg(id));
    game_runtime::spawn_tracked(id, pending, instance, || {
        cmd.spawn()
            .map_err(|e| AppError::process(format!("Failed to launch game: {e}")))
    })
}

fn prepare_launch_dir(args: &LaunchModdedArgs) -> AppResult<(PathBuf, LaunchInstance)> {
    let profile_dir = PathBuf::from(&args.profile_path);
    if !args.allow_instance_copy {
        return Ok((profile_dir, LaunchInstance::default()));
    }

    let used = game_runtime::used_instance_slots(&args.profile_id);
    let slot = (0..).find(|slot| !used.contains(slot)).expect("free slot");
    if slot == 0 {
        return Ok((profile_dir, LaunchInstance::default()));
    }

    let copy = profile_instance_service::create(&args.profile_id, &profile_dir, slot)?;
    Ok((
        copy.clone(),
        LaunchInstance {
            slot,
            temporary_dir: Some(copy),
        },
    ))
}

fn rebase_into_launch_dir(path: &str, profile_dir: &Path, launch_dir: &Path) -> String {
    Path::new(path)
        .strip_prefix(profile_dir)
        .map(|tail| launch_dir.join(tail))
        .unwrap_or_else(|_| PathBuf::from(path))
        .to_string_lossy()
        .to_string()
}

pub const STEAM_APP_ID: &str = "945360";

/// Lets Steamworks identify the app when the Steam client didn't start the exe.
fn ensure_steam_appid_file(game_dir: &Path) {
    let path = game_dir.join("steam_appid.txt");
    if fs::read_to_string(&path).is_ok_and(|s| s.trim() == STEAM_APP_ID) {
        return;
    }
    if let Err(e) = fs::write(&path, STEAM_APP_ID) {
        debug!("failed to write {}: {e}", path.display());
    }
}

/// `steam -applaunch` can't pass `--doorstop-*` arguments, so Steam launches
/// configure Doorstop through this file.
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

#[cfg(target_os = "linux")]
fn clear_doorstop_ini(game_dir: &Path) -> AppResult<()> {
    fs::write(
        game_dir.join("doorstop_config.ini"),
        "[General]\nenabled = false\n",
    )?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn spawn_steam() -> AppResult<()> {
    use std::os::unix::process::CommandExt;
    let mut child = Command::new("steam")
        .arg("-applaunch")
        .arg(STEAM_APP_ID)
        .process_group(0)
        .spawn()
        .map_err(|e| AppError::process(format!("Failed to launch via Steam: {e}")))?;
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

/// The Doorstop proxy needs wine to prefer the native `winhttp`. Launches we
/// spawn set `WINEDLLOVERRIDES`; `steam -applaunch` can't, so the override goes
/// in the prefix registry. Wine merges the duplicate section on load.
#[cfg(target_os = "linux")]
fn ensure_winhttp_dll_override(compat_data_path: &str) -> AppResult<()> {
    const OVERRIDE: &str = r#""winhttp"="native,builtin""#;

    let user_reg = PathBuf::from(compat_data_path).join("pfx").join("user.reg");
    let registry = fs::read_to_string(&user_reg).map_err(|e| {
        AppError::validation(format!(
            "Proton prefix not found at {} ({e}). Start Among Us from Steam once to create it, then launch again.",
            user_reg.display()
        ))
    })?;

    if registry.lines().any(|line| line.trim() == OVERRIDE) {
        return Ok(());
    }

    // A partial write would corrupt the prefix, so swap the file in.
    let temporary_path = user_reg.with_extension("reg.tmp");
    fs::write(
        &temporary_path,
        format!("{registry}\n[Software\\\\Wine\\\\DllOverrides] 0\n{OVERRIDE}\n"),
    )?;
    fs::rename(&temporary_path, &user_reg)?;
    info!("added winhttp DLL override to {}", user_reg.display());
    Ok(())
}

#[cfg(target_os = "linux")]
fn launch_modded_via_steam(
    args: &LaunchModdedArgs,
    game_dir: &Path,
    compat_data_path: &str,
    pending: &PendingLaunch,
) -> AppResult<()> {
    prepare_linux_winhttp_proxy(game_dir, &args.profile_path)?;
    ensure_winhttp_dll_override(compat_data_path)?;
    write_doorstop_ini(
        game_dir,
        &game_path(&args.bepinex_dll),
        &game_path(&args.dotnet_dir),
        &game_path(&args.coreclr_path),
    )?;
    game_runtime::start_steam_tracked(Some(pending), spawn_steam)
}

/// Fails without launching if Stop cancels `pending` first.
fn launch_modded(args: LaunchModdedArgs, pending: &PendingLaunch) -> AppResult<()> {
    info!("game_launch_modded: game_exe={}", args.game_exe);

    let game_dir = PathBuf::from(&args.game_exe)
        .parent()
        .ok_or_else(|| AppError::validation("Invalid game path"))?
        .to_path_buf();

    #[cfg(target_os = "linux")]
    if matches!(args.runner, LinuxRunner::Steam { .. }) {
        super::steam_installation::validate_directory(
            &game_dir,
            &super::finder_service::linux_steam_roots(),
        )?;
    }

    let _launch_guard = LAUNCH_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    pending.check()?;

    // The Steam client runs the first instance (online play and audio need
    // it); later ones replay its launch below.
    #[cfg(target_os = "linux")]
    if let LinuxRunner::Steam { compat_data_path } = &args.runner {
        let steam_running = steam_game_running(|| pending.check().is_err());
        pending.check()?;
        if !steam_running {
            return launch_modded_via_steam(&args, &game_dir, compat_data_path, pending);
        }
    }

    let profile_dir = PathBuf::from(&args.profile_path);
    let (launch_dir, instance) = prepare_launch_dir(&args)?;
    let bepinex_dll = rebase_into_launch_dir(&args.bepinex_dll, &profile_dir, &launch_dir);
    let dotnet_dir = rebase_into_launch_dir(&args.dotnet_dir, &profile_dir, &launch_dir);
    let coreclr_path = rebase_into_launch_dir(&args.coreclr_path, &profile_dir, &launch_dir);
    let launch_dir_str = launch_dir.to_string_lossy().to_string();

    let copy_to_clean_up = instance.temporary_dir.clone();
    let result = spawn_modded(
        &args,
        &game_dir,
        &launch_dir_str,
        LaunchPaths {
            bepinex_dll,
            dotnet_dir,
            coreclr_path,
        },
        instance,
        pending,
    );
    if result.is_err()
        && let Some(directory) = &copy_to_clean_up
    {
        profile_instance_service::release(directory);
    }
    result
}

struct LaunchPaths {
    bepinex_dll: String,
    dotnet_dir: String,
    coreclr_path: String,
}

fn spawn_modded(
    args: &LaunchModdedArgs,
    game_dir: &Path,
    launch_dir: &str,
    paths: LaunchPaths,
    instance: LaunchInstance,
    pending: &PendingLaunch,
) -> AppResult<()> {
    #[cfg(windows)]
    set_dll_directory(launch_dir)?;

    #[cfg(target_os = "linux")]
    prepare_linux_winhttp_proxy(game_dir, launch_dir)?;

    let mut cmd = build_game_command(
        &args.game_exe,
        #[cfg(target_os = "linux")]
        &args.runner,
    )?;

    cmd.current_dir(game_dir)
        .args(["--doorstop-enabled", "true"])
        .args(["--doorstop-target-assembly", &game_path(&paths.bepinex_dll)])
        .args(["--doorstop-clr-corlib-dir", &game_path(&paths.dotnet_dir)])
        .args([
            "--doorstop-clr-runtime-coreclr-path",
            &game_path(&paths.coreclr_path),
        ]);

    #[cfg(target_os = "linux")]
    {
        cmd.env("WINEDLLOVERRIDES", "winhttp=n,b");
    }

    attach_epic_launch_args(&mut cmd, args.platform)?;
    launch_process(cmd, Some(pending), instance)
}

pub fn launch_vanilla(args: LaunchVanillaArgs) -> AppResult<()> {
    info!("game_launch_vanilla: game_exe={}", args.game_exe);

    let game_dir = PathBuf::from(&args.game_exe)
        .parent()
        .ok_or_else(|| AppError::validation("Invalid game path"))?
        .to_path_buf();

    #[cfg(target_os = "linux")]
    if matches!(args.runner, LinuxRunner::Steam { .. }) {
        super::steam_installation::validate_directory(
            &game_dir,
            &super::finder_service::linux_steam_roots(),
        )?;
    }

    let _launch_guard = LAUNCH_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    #[cfg(target_os = "linux")]
    if matches!(args.runner, LinuxRunner::Steam { .. }) && !steam_game_running(|| false) {
        clear_doorstop_ini(&game_dir)?;
        return game_runtime::start_steam_tracked(None, spawn_steam);
    }

    let mut cmd = build_game_command(
        &args.game_exe,
        #[cfg(target_os = "linux")]
        &args.runner,
    )?;

    cmd.current_dir(&game_dir)
        .args(["--doorstop-enabled", "false"]);

    attach_epic_launch_args(&mut cmd, args.platform)?;
    launch_process(cmd, None, LaunchInstance::default())
}

pub fn launch_vanilla_from_settings() -> AppResult<()> {
    let settings = core_service::get_settings()?;
    let game = &settings.game;
    let game_exe = game.executable()?;

    #[cfg(windows)]
    if matches!(game.game_platform, GamePlatform::Xbox) {
        let app_id = xbox_service::get_xbox_app_id()?;
        xbox_service::cleanup_xbox_files(game_exe.parent().expect("game_exe has a parent"))?;
        return xbox_service::launch_xbox(&app_id);
    }

    if matches!(game.game_platform, GamePlatform::Steam) {
        ensure_steam_appid_file(game_exe.parent().expect("game_exe has a parent"));
    }
    allow_multiple_game_processes(&settings, game);

    #[cfg(target_os = "linux")]
    let runner = build_linux_runner(game)?;

    launch_vanilla(LaunchVanillaArgs {
        game_exe: game_exe.to_string_lossy().to_string(),
        platform: game.game_platform,
        #[cfg(target_os = "linux")]
        runner,
    })
}

/// The configured compat data path, else derived from the game's location.
#[cfg(target_os = "linux")]
fn steam_compat_data_path(game: &GameSetup) -> AppResult<String> {
    let configured = game.linux_proton_compat_data_path.trim();
    if !configured.is_empty() {
        return Ok(configured.to_string());
    }

    crate::backend::services::finder_service::proton_compat_data_path(Path::new(
        &game.among_us_path,
    ))
    .map(|path| path.to_string_lossy().to_string())
    .ok_or_else(|| {
        AppError::validation(format!(
            "Could not find the Proton prefix for {}. Set the compatibility data path in Settings.",
            game.among_us_path
        ))
    })
}

#[cfg(target_os = "linux")]
fn build_linux_runner(game: &GameSetup) -> AppResult<LinuxRunner> {
    use crate::backend::services::core_service::LinuxRunnerKind;

    if matches!(game.linux_runner_kind, LinuxRunnerKind::Steam) {
        if game.game_platform != GamePlatform::Steam {
            return Err(AppError::validation(format!(
                "The Steam runner can only launch the Steam version of Among Us. Choose Wine or Proton in Settings to launch the {} version.",
                game.game_platform.display_name()
            )));
        }
        return Ok(LinuxRunner::Steam {
            compat_data_path: steam_compat_data_path(game)?,
        });
    }

    let binary = game.linux_runner_binary.trim();
    if binary.is_empty() {
        return Err(AppError::validation(
            "Linux runner binary is required in Settings.",
        ));
    }
    Ok(match game.linux_runner_kind {
        LinuxRunnerKind::Wine => LinuxRunner::Wine {
            binary: binary.to_string(),
            prefix: game.linux_wine_prefix.clone(),
        },
        LinuxRunnerKind::Proton => LinuxRunner::Proton {
            binary: binary.to_string(),
            compat_data_path: game.linux_proton_compat_data_path.clone(),
            steam_client_path: game.linux_proton_steam_client_path.clone(),
            use_steam_run: game.linux_proton_use_steam_run,
        },
        LinuxRunnerKind::Steam => unreachable!("handled above"),
    })
}

/// Unity won't start a second process while boot.config has `single-instance`.
/// Checked every launch because game updates put the entry back.
fn allow_multiple_game_processes(settings: &core_service::AppSettings, game: &GameSetup) {
    if settings.allow_multi_instance_launch
        && let Err(e) = core_service::remove_single_instance_from_boot_config(&game.among_us_path)
    {
        warn!("failed to clear single-instance from boot.config: {e}");
    }
}

pub fn launch_modded_for_profile(profile: ProfileEntry) -> AppResult<()> {
    let pending = game_runtime::begin_launch(&profile.id);
    let settings = core_service::get_settings()?;
    let game = profile.installation(&settings)?;
    if profile.needs_bepinex(&settings) {
        return Err(AppError::validation(
            "Install BepInEx for the selected installation before launching.",
        ));
    }
    let game_exe = game.executable()?;

    let runtime = profile.bepinex_runtime();
    let bepinex_dll = runtime.assembly_path();
    let dotnet_dir = runtime.dotnet_dir();
    let coreclr_path = runtime.coreclr_path();

    #[cfg(windows)]
    if matches!(game.game_platform, GamePlatform::Xbox) {
        let app_id = xbox_service::get_xbox_app_id()?;
        let game_dir = game_exe.parent().expect("game_exe has a parent");
        xbox_service::prepare_xbox_launch(runtime.root(), game_dir)?;
        pending.check()?;
        xbox_service::launch_xbox(&app_id)?;
        if let Err(e) = profile_service::update_last_launched(&profile.id) {
            debug!("update_last_launched failed for Xbox launch: {e}");
        }
        return Ok(());
    }

    if matches!(game.game_platform, GamePlatform::Steam) {
        ensure_steam_appid_file(game_exe.parent().expect("game_exe has a parent"));
    }

    allow_multiple_game_processes(&settings, game);

    #[cfg(target_os = "linux")]
    let runner = build_linux_runner(game)?;

    launch_modded(
        LaunchModdedArgs {
            game_exe: game_exe.to_string_lossy().to_string(),
            profile_id: profile.id.clone(),
            profile_path: profile.path.clone(),
            bepinex_dll: bepinex_dll.to_string_lossy().to_string(),
            dotnet_dir: dotnet_dir.to_string_lossy().to_string(),
            coreclr_path: coreclr_path.to_string_lossy().to_string(),
            platform: game.game_platform,
            allow_instance_copy: settings.allow_multi_instance_launch,
            #[cfg(target_os = "linux")]
            runner,
        },
        &pending,
    )
}

/// Which profile a lobby launch runs from.
#[derive(Clone, PartialEq)]
pub enum LobbyLaunchTarget {
    Existing(String),
    /// A fresh throwaway profile, deleted once its game exits. Xbox launches
    /// aren't tracked, so there it waits for the next startup's cleanup.
    Temporary,
}

/// Launch into a lobby: resolve `target` to a profile, install the lobby's
/// required `mods` into it, point Among Us at the lobby's region and launch.
/// A temporary profile is deleted again if the launch fails, since no game
/// exit will clean it up. Returns a summary for the user. Blocking; run on the
/// background executor.
pub fn launch_into_lobby(
    target: LobbyLaunchTarget,
    mods: &[LobbyMod],
    server_host: &str,
    server_port: u16,
) -> AppResult<String> {
    // Mods with an id but no version can't be installed (we don't know
    // which version), but still count toward the "skipped" total so the
    // launch summary doesn't silently omit them.
    let mut required: Vec<InstallModInput> = Vec::new();
    let mut versionless = 0usize;
    for m in mods {
        let Some(id) = m.id.clone() else { continue };
        match m.version.clone() {
            Some(version) => required.push(InstallModInput {
                mod_id: id,
                version,
            }),
            None => versionless += 1,
        }
    }

    let profile = match &target {
        LobbyLaunchTarget::Existing(id) => profile_service::get_profile_by_id(id)?
            .ok_or_else(|| AppError::validation(t!("lobbies.profile_gone").to_string()))?,
        LobbyLaunchTarget::Temporary => create_temp_profile()?,
    };
    let temp_profile_id = (target == LobbyLaunchTarget::Temporary).then(|| profile.id.clone());
    let launched =
        launch_into_lobby_for_profile(profile, required, versionless, server_host, server_port);
    if launched.is_err()
        && let Some(id) = temp_profile_id
        && let Err(e) = profile_service::delete_profile(&id)
    {
        warn!("failed to clean up temp profile {id} after launch error: {e}");
    }
    launched
}

/// Install the lobby's required mods into `profile`, point Among Us at the
/// lobby's region, and launch. `versionless` is the count of required mods
/// the lobby sent with no version (uninstallable, but still reported as
/// skipped rather than silently dropped).
fn launch_into_lobby_for_profile(
    profile: ProfileEntry,
    required: Vec<InstallModInput>,
    versionless: usize,
    server_host: &str,
    server_port: u16,
) -> AppResult<String> {
    profile_service::install_bepinex_for_profile(&profile.id)?;

    let mut skipped = versionless;
    let mut failed = 0usize;
    if !required.is_empty() {
        let (installable, unresolved) = mod_install_service::plan_lobby_mods(&required);
        skipped += unresolved.len();
        // Skip mods already present at the exact version the lobby wants.
        let missing: Vec<InstallModInput> = installable
            .into_iter()
            .filter(|m| {
                !profile
                    .mods
                    .iter()
                    .any(|p| p.mod_id == m.mod_id && p.version == m.version)
            })
            .collect();
        // Install one mod at a time: install_mods_for_profile rolls back its
        // whole batch on a single failure, which is right for one coherent
        // "install this mod" user action but wrong here — a lobby launch
        // wants each required mod to be independently best-effort, so one
        // flaky download doesn't sink mods that already succeeded.
        for item in missing {
            let mod_id = item.mod_id.clone();
            if let Err(e) = mod_install_service::install_mods_for_profile(
                &profile.id,
                std::slice::from_ref(&item),
            ) {
                warn!("failed to install mod {mod_id} for lobby launch: {e}");
                failed += 1;
            }
        }
    }

    let region_set =
        region_service::select_region_by_host_port(server_host, server_port).unwrap_or(false);

    // Reload so the launch sees the freshly installed BepInEx / mods.
    let profile = profile_service::get_profile_by_id(&profile.id)?
        .ok_or_else(|| AppError::validation(t!("lobbies.profile_gone_launch").to_string()))?;
    launch_modded_for_profile(profile)?;

    let mut summary = if region_set {
        t!("lobbies.launched_region_set").to_string()
    } else {
        t!("lobbies.launched").to_string()
    };
    if skipped > 0 {
        summary.push_str(t!("lobbies.skipped_catalog", count = skipped).as_ref());
    }
    if failed > 0 {
        summary.push_str(t!("lobbies.skipped_failed", count = failed).as_ref());
    }
    Ok(summary)
}

/// Create a fresh throwaway profile for a one-off lobby launch, uniquely named
/// so repeated temporary launches don't collide. The backend deletes it once
/// the launched game exits.
fn create_temp_profile() -> AppResult<ProfileEntry> {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    profile_service::create_temporary_profile(&format!("Temporary Lobby {millis}"))
}
