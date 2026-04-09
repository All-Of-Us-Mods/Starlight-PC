import type { QueryClient } from "@tanstack/svelte-query";
import type { AppSettings } from "$lib/features/settings/schema";
import { resolveApiUrl } from "$lib/infra/http/starlight-api";
import { rustInvoke } from "$lib/infra/rust/invoke";
import { gameState } from "../state/game-state.svelte";
import { modQueries } from "$lib/features/mods/queries";
import type { ModVersionInfo } from "$lib/features/mods/schema";
import {
  getProfileById,
  invalidateProfileAndDiskQueries,
  resolveProfilePluginPath,
} from "./profile-files.service";
import { listenForBepInExProgress, listenForModDownloadProgress } from "./profile-events.service";

export type InstallArgs = {
  profileId: string;
  mods: Array<{ modId: string; version: string }>;
};

type PreviousModState = Map<string, { version: string; file?: string } | null>;

export type InstalledModResult = { mod_id: string; version: string; file_name: string };

type DownloadTarget = {
  url: string;
  fileName: string;
  checksum?: string;
};

function inferFileNameFromUrl(url: string, fallback: string): string {
  try {
    const parsed = new URL(url);
    const segment = parsed.pathname.split("/").filter(Boolean).at(-1);
    if (segment) return decodeURIComponent(segment);
  } catch {
    // Best-effort fallback for non-absolute paths.
  }

  return fallback;
}

const bepinexInstallInFlight = new Set<string>();
const modsInstallInFlight = new Set<string>();

function resolveDownloadTarget(
  modId: string,
  version: string,
  versionInfo: ModVersionInfo,
  platform: AppSettings["game_platform"],
): DownloadTarget {
  const legacyPath = `/api/v3/mods/${modId}/versions/${version}/file`;
  const architectureFallbacks = platform === "epic" ? ["x64", "x86"] : ["x86", "x64"];

  const platforms = versionInfo.platforms ?? [];
  for (const arch of architectureFallbacks) {
    const entry = platforms.find(
      (candidate) => candidate.platform === "windows" && candidate.architecture === arch,
    );
    const downloadUrl = entry?.download_url ?? `${legacyPath}?platform=windows&arch=${arch}`;
    const resolvedUrl = resolveApiUrl(downloadUrl);
    return {
      url: resolvedUrl,
      fileName: entry?.file_name ?? inferFileNameFromUrl(resolvedUrl, `${modId}-${version}.dll`),
      checksum: entry?.checksum,
    };
  }

  const supportedPlatforms = versionInfo.supported_platforms ?? [];
  for (const supported of supportedPlatforms) {
    const [supportedPlatform, supportedArch] = supported.split("_");
    if (!supportedPlatform || !supportedArch) continue;
    if (supportedPlatform !== "windows") continue;

    const downloadUrl = `${legacyPath}?platform=${supportedPlatform}&arch=${supportedArch}`;
    const resolvedUrl = resolveApiUrl(downloadUrl);
    return {
      url: resolvedUrl,
      fileName: inferFileNameFromUrl(resolvedUrl, `${modId}-${version}.dll`),
    };
  }

  const fallbackUrl = resolveApiUrl(`${legacyPath}?platform=windows&arch=x86`);
  return {
    url: fallbackUrl,
    fileName: inferFileNameFromUrl(fallbackUrl, `${modId}-${version}.dll`),
  };
}

async function rollbackInstalledMods(
  profileId: string,
  profilePath: string,
  installed: InstalledModResult[],
  persisted: InstalledModResult[],
  previousByModId: PreviousModState,
) {
  await Promise.all(
    persisted.toReversed().map(async (item) => {
      const previous = previousByModId.get(item.mod_id);
      if (previous?.file) {
        await rustInvoke("profiles_add_mod", {
          profileId,
          modId: item.mod_id,
          version: previous.version,
          file: previous.file,
        }).catch((error) => {
          console.warn("[rollback] Failed to restore mod metadata", {
            profileId,
            modId: item.mod_id,
            error,
          });
        });
        return;
      }

      await rustInvoke("profiles_remove_mod", {
        profileId,
        modId: item.mod_id,
      }).catch((error) => {
        console.warn("[rollback] Failed to remove rolled-back mod metadata", {
          profileId,
          modId: item.mod_id,
          error,
        });
      });
    }),
  );

  await Promise.all(
    installed.toReversed().map((item) =>
      rustInvoke("profiles_delete_mod_file", { profilePath, fileName: item.file_name }).catch(
        (error) => {
          console.warn("[rollback] Failed to delete rolled-back mod file", {
            profilePath,
            fileName: item.file_name,
            error,
          });
        },
      ),
    ),
  );
}

export async function installBepInExForProfile(profileId: string) {
  if (bepinexInstallInFlight.has(profileId)) {
    throw new Error("BepInEx install already in progress for this profile");
  }
  bepinexInstallInFlight.add(profileId);

  let unlisten: (() => void) | undefined;
  let succeeded = false;
  try {
    unlisten = await listenForBepInExProgress(profileId);
    await rustInvoke("profiles_install_bepinex", { profileId });
    succeeded = true;
  } catch (error) {
    const message = error instanceof Error ? error.message : "Unknown error";
    gameState.setBepInExError(profileId, message);
    throw error;
  } finally {
    bepinexInstallInFlight.delete(profileId);
    unlisten?.();
    if (succeeded) {
      gameState.clearBepInExProgress(profileId);
    }
  }
}

export async function installModsForProfile(
  queryClient: QueryClient,
  args: InstallArgs,
): Promise<InstalledModResult[]> {
  if (modsInstallInFlight.has(args.profileId)) {
    throw new Error("An install is already in progress for this profile");
  }
  modsInstallInFlight.add(args.profileId);

  let unlistenModDownload: (() => void) | undefined;
  let failed = false;
  const installed: InstalledModResult[] = [];

  try {
    const settings = await rustInvoke("core_get_settings");
    const profile = await getProfileById(args.profileId);
    if (!profile) {
      throw new Error(`Profile '${args.profileId}' not found`);
    }

    unlistenModDownload = await listenForModDownloadProgress();

    const previousByModId: PreviousModState = new Map();
    for (const item of args.mods) {
      const previous = profile.mods.find((entry) => entry.mod_id === item.modId);
      previousByModId.set(
        item.modId,
        previous ? { version: previous.version, file: previous.file ?? undefined } : null,
      );
    }

    const persisted: InstalledModResult[] = [];
    const replacedFilesToDelete = new Set<string>();

    /* eslint-disable no-await-in-loop */
    for (const item of args.mods) {
      try {
        const versionInfo = await queryClient.fetchQuery(
          modQueries.versionInfo(item.modId, item.version),
        );
        const target = resolveDownloadTarget(
          item.modId,
          item.version,
          versionInfo,
          settings.game_platform,
        );
        const destination = await resolveProfilePluginPath(profile.path, target.fileName);

        await rustInvoke("modding_mod_download", {
          modId: item.modId,
          url: target.url,
          destination,
          ...(target.checksum ? { expectedChecksum: target.checksum } : {}),
        });

        installed.push({
          mod_id: item.modId,
          version: item.version,
          file_name: target.fileName,
        });

        await rustInvoke("profiles_add_mod", {
          profileId: args.profileId,
          modId: item.modId,
          version: item.version,
          file: target.fileName,
        });

        persisted.push({
          mod_id: item.modId,
          version: item.version,
          file_name: target.fileName,
        });

        const previous = previousByModId.get(item.modId);
        if (previous?.file && previous.file !== target.fileName) {
          replacedFilesToDelete.add(previous.file);
        }
      } catch (error) {
        await rollbackInstalledMods(
          args.profileId,
          profile.path,
          installed,
          persisted,
          previousByModId,
        );
        throw error;
      }
    }
    /* eslint-enable no-await-in-loop */

    await Promise.all(
      Array.from(replacedFilesToDelete).map((fileName) =>
        rustInvoke("profiles_delete_mod_file", {
          profilePath: profile.path,
          fileName,
        }).catch((error) => {
          console.warn("[installMods] Failed to delete replaced mod file", {
            profilePath: profile.path,
            fileName,
            error,
          });
        }),
      ),
    );

    return installed;
  } catch (error) {
    failed = true;
    const message = error instanceof Error ? error.message : "Unknown error";
    for (const item of args.mods) {
      if (!installed.some((entry) => entry.mod_id === item.modId)) {
        gameState.setModDownloadError(item.modId, message);
      }
    }
    throw error;
  } finally {
    modsInstallInFlight.delete(args.profileId);
    unlistenModDownload?.();
    if (!failed) {
      for (const item of args.mods) {
        gameState.clearModDownload(item.modId);
      }
    }
  }
}

export async function invalidateAfterModInstall(
  queryClient: QueryClient,
  args: { profileId: string; profilePath?: string },
) {
  await invalidateProfileAndDiskQueries(queryClient, args);
}
