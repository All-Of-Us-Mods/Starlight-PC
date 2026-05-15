import type { QueryClient } from "@tanstack/svelte-query";
import type { AppSettings } from "$lib/features/settings/schema";
import { settingsQueryKey } from "$lib/features/settings/settings-keys";
import { closeCurrentWindow } from "$lib/infra/tauri/window";
import { rustInvoke } from "$lib/infra/rust/invoke";
import type { LinuxRunnerArgs } from "$lib/infra/rust/commands";
import { epicAuthService } from "$lib/features/settings/services/epic-auth.service";
import { platform } from "@tauri-apps/plugin-os";
import type { Profile } from "../schema";
import {
  assertPathExists,
  resolveBepInExDllPath,
  resolveCoreClrPath,
  resolveDotnetDir,
  resolveGameExecutablePath,
} from "./profile-files.service";

function getLinuxRunnerArgs(settings: AppSettings): LinuxRunnerArgs | undefined {
  if (platform() !== "linux") return undefined;

  const binary = settings.linux_runner_binary.trim();
  if (!binary) {
    throw new Error("Linux runner binary is required in Settings.");
  }

  if (settings.linux_runner_kind === "wine") {
    const prefix = settings.linux_wine_prefix.trim();
    if (!prefix) {
      throw new Error("Wine prefix is required in Settings.");
    }
    return { kind: "wine", binary, prefix };
  }

  const compatDataPath = settings.linux_proton_compat_data_path.trim();
  const steamClientPath = settings.linux_proton_steam_client_path.trim();
  if (!compatDataPath || !steamClientPath) {
    throw new Error("Proton compat data path and Steam client path are required in Settings.");
  }

  return {
    kind: "proton",
    binary,
    compatDataPath,
    steamClientPath,
    useSteamRun: settings.linux_proton_use_steam_run,
  };
}

export async function ensureEpicLogin(): Promise<void> {
  await epicAuthService.ensureLoggedIn();
}

async function ensureXboxAppId(settings: AppSettings, queryClient?: QueryClient): Promise<string> {
  let appId = settings.xbox_app_id?.trim() ?? "";
  if (!appId) {
    appId = await rustInvoke("game_xbox_get_app_id");
    await rustInvoke("core_update_settings", { updates: { xbox_app_id: appId } });
    queryClient?.setQueryData<AppSettings | undefined>(settingsQueryKey, (current) =>
      current ? { ...current, xbox_app_id: appId } : current,
    );
  }

  return appId;
}

export async function launchXboxProfile(
  settings: AppSettings,
  profile: Profile,
  queryClient?: QueryClient,
) {
  const appId = await ensureXboxAppId(settings, queryClient);
  await rustInvoke("game_xbox_prepare_launch", {
    gameDir: settings.among_us_path,
    profilePath: profile.path,
  });
  await rustInvoke("game_xbox_launch", {
    appId,
    profileId: profile.id,
  });
}

export async function launchXboxVanilla(settings: AppSettings, queryClient?: QueryClient) {
  const appId = await ensureXboxAppId(settings, queryClient);
  await rustInvoke("game_xbox_cleanup", { gameDir: settings.among_us_path });
  await rustInvoke("game_xbox_launch", { appId, profileId: null });
}

export async function launchModdedProfile(profile: Profile, settings: AppSettings) {
  const gameExe = await resolveGameExecutablePath(settings.among_us_path);
  await assertPathExists(gameExe, "Among Us.exe not found at configured path");

  const bepinexDll = await resolveBepInExDllPath(profile.path);
  await assertPathExists(
    bepinexDll,
    "BepInEx DLL not found. Please wait for installation to complete.",
  );

  const dotnetDir = await resolveDotnetDir(profile.path);
  const coreclrPath = await resolveCoreClrPath(dotnetDir);
  await assertPathExists(
    coreclrPath,
    "dotnet runtime not found. Please wait for installation to complete.",
  );

  const runner = getLinuxRunnerArgs(settings);

  await rustInvoke("game_launch_modded", {
    gameExe,
    profileId: profile.id,
    profilePath: profile.path,
    bepinexDll,
    dotnetDir,
    coreclrPath,
    platform: settings.game_platform,
    runner,
  });
}

export async function launchVanillaGame(settings: AppSettings) {
  const gameExe = await resolveGameExecutablePath(settings.among_us_path);
  await assertPathExists(gameExe, "Among Us.exe not found at configured path");
  const runner = getLinuxRunnerArgs(settings);
  await rustInvoke("game_launch_vanilla", {
    gameExe,
    platform: settings.game_platform,
    runner,
  });
}

export async function stopProfileDesktopInstances(profileId: string) {
  return await rustInvoke("game_stop_profile_instances", { profileId });
}

export async function stopAllDesktopInstances() {
  return await rustInvoke("game_stop_all_instances");
}

export async function recordLastLaunched(profileId: string) {
  try {
    await rustInvoke("profiles_update_last_launched", { profileId });
  } catch {
    // Best-effort bookkeeping; launch already succeeded.
  }
}

export async function closeWindowAfterLaunch(closeOnLaunch: boolean) {
  if (closeOnLaunch) {
    await closeCurrentWindow();
  }
}
