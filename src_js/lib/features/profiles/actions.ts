import type { QueryClient } from "@tanstack/svelte-query";
import { gameState } from "./state/game-state.svelte";
import type { Profile, ProfileIconSelection, UnifiedMod } from "./schema";
import { profileDiskFilesKey, profileUnifiedModsKey, profilesQueryKey } from "./profile-keys";
import { rustInvoke } from "$lib/infra/rust/invoke";
import {
  invalidateProfileAndDiskQueries,
  getProfilePathFromCache,
} from "./services/profile-files.service";
import {
  closeWindowAfterLaunch,
  ensureEpicLogin,
  launchModdedProfile,
  launchVanillaGame,
  launchXboxProfile,
  launchXboxVanilla,
  stopAllDesktopInstances,
  stopProfileDesktopInstances,
  recordLastLaunched,
} from "./services/profile-launch.service";
import {
  installBepInExForProfile,
  type InstallArgs,
  invalidateAfterModInstall,
  installModsForProfile,
} from "./services/profile-install.service";
import { removeMissingMods, removeUnifiedMod } from "./services/profile-mods.service";
import { withProfileMutationTracking } from "./services/profile-mutations.service";
import { resolveProfileShortcutIconBytes } from "./services/profile-shortcut.service";
import { showError } from "$lib/utils/toast";

let launchInFlight = false;

export const profileActions = {
  create: (queryClient: QueryClient) => ({
    mutationFn: (name: string) =>
      withProfileMutationTracking(async () => {
        const profile = await rustInvoke("profiles_create", { name });
        // Track the background install so the watcher stays paused
        void withProfileMutationTracking(() => installBepInExForProfile(profile.id))
          .catch((error) => {
            console.error("[profiles] Background BepInEx install failed", error);
          })
          .finally(() => {
            void queryClient.invalidateQueries({ queryKey: profilesQueryKey });
          });
        return profile;
      }),
    onSettled: () => {
      void queryClient.invalidateQueries({ queryKey: profilesQueryKey });
    },
  }),

  delete: (queryClient: QueryClient) => ({
    mutationFn: (profileId: string) =>
      withProfileMutationTracking(() => rustInvoke("profiles_delete", { profileId })),
    onSettled: async () => {
      await queryClient.invalidateQueries({ queryKey: profilesQueryKey });
    },
  }),

  rename: (queryClient: QueryClient) => ({
    mutationFn: (args: { profileId: string; newName: string }) =>
      withProfileMutationTracking(() => rustInvoke("profiles_rename", args)),
    onSettled: async () => {
      await queryClient.invalidateQueries({ queryKey: profilesQueryKey });
    },
  }),

  updateIcon: (queryClient: QueryClient) => ({
    mutationFn: (args: { profileId: string; selection: ProfileIconSelection }) =>
      withProfileMutationTracking(() => rustInvoke("profiles_update_icon", args)),
    onSettled: async (_data: unknown, _error: unknown, args: { profileId: string }) => {
      await invalidateProfileAndDiskQueries(queryClient, args);
      await queryClient.invalidateQueries({
        predicate: (query) =>
          query.queryKey[0] === "profiles" && query.queryKey[1] === "binary-file",
      });
    },
  }),

  addMod: (queryClient: QueryClient) => ({
    mutationFn: (args: { profileId: string; modId: string; version: string; file: string }) =>
      withProfileMutationTracking(() => rustInvoke("profiles_add_mod", args)),
    onSettled: async (_data: unknown, _error: unknown, args: { profileId: string }) => {
      await invalidateProfileAndDiskQueries(queryClient, args);
    },
  }),

  removeMod: (queryClient: QueryClient) => ({
    mutationFn: (args: { profileId: string; modId: string }) =>
      withProfileMutationTracking(() => rustInvoke("profiles_remove_mod", args)),
    onSettled: async (_data: unknown, _error: unknown, args: { profileId: string }) => {
      await invalidateProfileAndDiskQueries(queryClient, args);
    },
  }),

  deleteUnifiedMod: (queryClient: QueryClient) => ({
    mutationFn: (args: { profileId: string; mod: UnifiedMod }) =>
      withProfileMutationTracking(() => removeUnifiedMod(args.profileId, args.mod)),
    onMutate: async (args: { profileId: string; mod: UnifiedMod }) => {
      const unifiedKey = profileUnifiedModsKey(args.profileId);
      const profilePath = getProfilePathFromCache(queryClient, args.profileId);
      const diskKey = profilePath ? profileDiskFilesKey(profilePath) : null;
      const targetMod = args.mod;

      await queryClient.cancelQueries({ queryKey: unifiedKey });
      if (diskKey) {
        await queryClient.cancelQueries({ queryKey: diskKey });
      }

      const previousUnified = queryClient.getQueryData<UnifiedMod[]>(unifiedKey);
      const previousDiskFiles = diskKey ? queryClient.getQueryData<string[]>(diskKey) : undefined;

      queryClient.setQueryData<UnifiedMod[]>(unifiedKey, (current) => {
        if (!current) return current;
        if (targetMod.source === "managed") {
          return current.filter(
            (item) => !(item.source === "managed" && item.mod_id === targetMod.mod_id),
          );
        }
        return current.filter(
          (item) => !(item.source === "custom" && item.file === targetMod.file),
        );
      });

      if (diskKey) {
        queryClient.setQueryData<string[]>(diskKey, (current) => {
          if (!current) return current;
          return current.filter((file) => file !== targetMod.file);
        });
      }

      return {
        unifiedKey,
        diskKey,
        previousUnified,
        previousDiskFiles,
      };
    },
    onError: (
      _error: unknown,
      _args: { profileId: string; mod: UnifiedMod },
      context:
        | {
            unifiedKey: readonly unknown[];
            diskKey: readonly unknown[] | null;
            previousUnified: UnifiedMod[] | undefined;
            previousDiskFiles: string[] | undefined;
          }
        | undefined,
    ) => {
      if (!context) return;
      queryClient.setQueryData(context.unifiedKey, context.previousUnified);
      if (context.diskKey) {
        queryClient.setQueryData(context.diskKey, context.previousDiskFiles);
      }
    },
    onSettled: async (_data: unknown, _error: unknown, args: { profileId: string }) => {
      await invalidateProfileAndDiskQueries(queryClient, args);
    },
  }),

  cleanupMissingMods: (queryClient: QueryClient) => ({
    mutationFn: (profileId: string) =>
      withProfileMutationTracking(() => removeMissingMods(profileId)),
    onSettled: async () => {
      await queryClient.invalidateQueries({ queryKey: profilesQueryKey });
    },
  }),

  updatePlayTime: (queryClient: QueryClient) => ({
    mutationFn: (args: { profileId: string; durationMs: number }) =>
      withProfileMutationTracking(() => rustInvoke("profiles_add_play_time", args)),
    onSettled: async () => {
      await queryClient.invalidateQueries({ queryKey: profilesQueryKey });
    },
  }),

  retryBepInExInstall: (queryClient: QueryClient) => ({
    mutationFn: (args: { profileId: string }) =>
      withProfileMutationTracking(() => installBepInExForProfile(args.profileId)),
    onSettled: async () => {
      await queryClient.invalidateQueries({ queryKey: profilesQueryKey });
    },
  }),

  exportZip: () => ({
    mutationFn: (args: { profileId: string; destination: string }) =>
      withProfileMutationTracking(() => rustInvoke("profiles_export_zip", args)),
  }),

  createDesktopShortcut: (queryClient: QueryClient) => ({
    mutationFn: (profile: Profile) =>
      withProfileMutationTracking(async () => {
        const iconBytes = await resolveProfileShortcutIconBytes(queryClient, profile);
        return rustInvoke("profiles_create_desktop_shortcut", {
          profileId: profile.id,
          iconBytes,
        });
      }),
  }),

  importMod: (queryClient: QueryClient) => ({
    mutationFn: (args: { profileId: string; sourcePath: string }) =>
      withProfileMutationTracking(() => rustInvoke("profiles_import_mod", args)),
    onMutate: async (args: { profileId: string; sourcePath: string }) => {
      const unifiedKey = profileUnifiedModsKey(args.profileId);
      const profilePath = getProfilePathFromCache(queryClient, args.profileId);
      const diskKey = profilePath ? profileDiskFilesKey(profilePath) : null;
      const importedFile = args.sourcePath.split(/[/\\]/).pop() ?? args.sourcePath;

      await queryClient.cancelQueries({ queryKey: unifiedKey });
      if (diskKey) {
        await queryClient.cancelQueries({ queryKey: diskKey });
      }

      const previousUnified = queryClient.getQueryData<UnifiedMod[]>(unifiedKey);
      const previousDiskFiles = diskKey ? queryClient.getQueryData<string[]>(diskKey) : undefined;

      queryClient.setQueryData<UnifiedMod[]>(unifiedKey, (current) => {
        if (!current) return current;
        const alreadyListed = current.some((item) => item.file === importedFile);
        if (alreadyListed) return current;
        return [...current, { source: "custom", file: importedFile }];
      });

      if (diskKey) {
        queryClient.setQueryData<string[]>(diskKey, (current) => {
          if (!current) return current;
          if (current.includes(importedFile)) return current;
          return [...current, importedFile];
        });
      }

      return {
        unifiedKey,
        diskKey,
        previousUnified,
        previousDiskFiles,
      };
    },
    onError: (
      _error: unknown,
      _args: { profileId: string; sourcePath: string },
      context:
        | {
            unifiedKey: readonly unknown[];
            diskKey: readonly unknown[] | null;
            previousUnified: UnifiedMod[] | undefined;
            previousDiskFiles: string[] | undefined;
          }
        | undefined,
    ) => {
      if (!context) return;
      queryClient.setQueryData(context.unifiedKey, context.previousUnified);
      if (context.diskKey) {
        queryClient.setQueryData(context.diskKey, context.previousDiskFiles);
      }
    },
    onSettled: async (_data: unknown, _error: unknown, args: { profileId: string }) => {
      await invalidateProfileAndDiskQueries(queryClient, args);
    },
  }),

  importZip: (queryClient: QueryClient) => ({
    mutationFn: (zipPath: string) =>
      withProfileMutationTracking(async () => {
        const profiles = await rustInvoke("profiles_import_zip", { zipPath });

        // Run installations in background
        void withProfileMutationTracking(async () => {
          await Promise.all(
            profiles.map(async (profile) => {
              try {
                if (!profile.bepinex_installed) {
                  await installBepInExForProfile(profile.id);
                }
                if (profile.mods && profile.mods.length > 0) {
                  try {
                    const installArgs = {
                      profileId: profile.id,
                      mods: profile.mods.map((m) => ({ modId: m.mod_id, version: m.version })),
                    };
                    await installModsForProfile(queryClient, installArgs);
                    void invalidateProfileAndDiskQueries(queryClient, installArgs);
                  } catch (e) {
                    console.error(
                      `[profiles] Failed to install imported mods for ${profile.id}`,
                      e,
                    );
                    showError(e, `Failed to install mods for imported profile "${profile.name}"`);
                  }
                }
              } catch (error) {
                console.error(
                  `[profiles] Post-import installation failed for ${profile.id}`,
                  error,
                );
                showError(error, `Post-import setup failed for "${profile.name}"`);
              } finally {
                void invalidateProfileAndDiskQueries(queryClient, { profileId: profile.id });
              }
            }),
          );
        });

        return profiles;
      }),
    onSettled: async () => {
      await queryClient.invalidateQueries({ queryKey: profilesQueryKey });
    },
  }),

  updateLastLaunched: (queryClient: QueryClient) => ({
    mutationFn: (profileId: string) =>
      withProfileMutationTracking(() => rustInvoke("profiles_update_last_launched", { profileId })),
    onSettled: async () => {
      await queryClient.invalidateQueries({ queryKey: profilesQueryKey });
    },
  }),

  installMods: (queryClient: QueryClient) => ({
    mutationFn: (args: InstallArgs) =>
      withProfileMutationTracking(() => installModsForProfile(queryClient, args)),
    onSettled: (_data: unknown, _error: unknown, args: InstallArgs) => {
      void invalidateAfterModInstall(queryClient, args);
    },
  }),

  launchProfile: (queryClient?: QueryClient) => ({
    mutationFn: async (profile: Profile) => {
      if (launchInFlight) {
        throw new Error("A launch is already in progress");
      }
      launchInFlight = true;
      try {
        const settings = await rustInvoke("core_get_settings");
        if (!settings.among_us_path?.trim()) {
          throw new Error("Among Us path not configured");
        }
        if (!settings.allow_multi_instance_launch && gameState.running) {
          throw new Error("An Among Us instance is already running");
        }
        if (settings.game_platform === "epic") {
          await ensureEpicLogin();
        }

        if (settings.game_platform === "xbox") {
          await launchXboxProfile(settings, profile, queryClient);
        } else {
          await launchModdedProfile(profile, settings);
        }
        await recordLastLaunched(profile.id);
        await closeWindowAfterLaunch(settings.close_on_launch);
      } finally {
        launchInFlight = false;
      }
    },
  }),

  launchVanilla: (queryClient?: QueryClient) => ({
    mutationFn: async () => {
      if (launchInFlight) {
        throw new Error("A launch is already in progress");
      }
      launchInFlight = true;
      try {
        const settings = await rustInvoke("core_get_settings");
        if (!settings.among_us_path?.trim()) {
          throw new Error("Among Us path not configured");
        }
        if (!settings.allow_multi_instance_launch && gameState.running) {
          throw new Error("An Among Us instance is already running");
        }
        if (settings.game_platform === "epic") {
          await ensureEpicLogin();
        }

        if (settings.game_platform === "xbox") {
          await launchXboxVanilla(settings, queryClient);
        } else {
          await launchVanillaGame(settings);
        }

        await closeWindowAfterLaunch(settings.close_on_launch);
      } finally {
        launchInFlight = false;
      }
    },
  }),

  stopProfileInstances: () => ({
    mutationFn: async (profileId: string) => {
      const stoppedCount = await stopProfileDesktopInstances(profileId);
      if (stoppedCount === 0 && gameState.isProfileRunning(profileId)) {
        throw new Error("No stoppable desktop instances found for this profile");
      }
      return stoppedCount;
    },
  }),

  stopAllInstances: () => ({
    mutationFn: async () => {
      const stoppedCount = await stopAllDesktopInstances();
      if (stoppedCount === 0 && gameState.running) {
        throw new Error("No stoppable desktop instances found");
      }
      return stoppedCount;
    },
  }),
};

export type CreateProfileAction = ReturnType<typeof profileActions.create>;
export type DeleteProfileAction = ReturnType<typeof profileActions.delete>;
