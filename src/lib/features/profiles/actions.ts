import type { QueryClient } from '@tanstack/svelte-query';
import { gameState } from './state/game-state.svelte';
import type { Profile, ProfileIconSelection, UnifiedMod } from './schema';
import { profilesQueryKey } from './profile-keys';
import { rustInvoke } from '$lib/infra/rust/invoke';
import { getProfileById, invalidateProfileAndDiskQueries } from './services/profile-files.service';
import {
	closeWindowAfterLaunch,
	ensureEpicLogin,
	launchModdedProfile,
	launchVanillaGame,
	launchXboxProfile,
	launchXboxVanilla,
	recordLastLaunched
} from './services/profile-launch.service';
import {
	installBepInExForProfile,
	type InstallArgs,
	invalidateAfterModInstall,
	installModsForProfile,
	type InstalledModResult
} from './services/profile-install.service';

let launchInFlight = false;
export const profileActions = {
	create: (queryClient: QueryClient) => ({
		mutationFn: async (name: string) => {
			const profile = await rustInvoke('profiles_create', { name });
			void installBepInExForProfile(profile.id)
				.catch((error) => {
					console.error('[profiles] Background BepInEx install failed', error);
				})
				.finally(() => {
					void queryClient.invalidateQueries({ queryKey: profilesQueryKey });
				});
			return profile;
		},
		onSuccess: (created: Profile) => {
			queryClient.setQueryData<Profile[] | undefined>(profilesQueryKey, (current) => {
				if (!current) return [created];
				const hasProfile = current.some((profile) => profile.id === created.id);
				return hasProfile
					? current.map((profile) => (profile.id === created.id ? created : profile))
					: [...current, created];
			});
		}
	}),

	delete: (queryClient: QueryClient) => ({
		mutationFn: (profileId: string) => rustInvoke('profiles_delete', { profileId }),
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: profilesQueryKey });
		}
	}),

	rename: (queryClient: QueryClient) => ({
		mutationFn: (args: { profileId: string; newName: string }) =>
			rustInvoke('profiles_rename', args),
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: profilesQueryKey });
		}
	}),

	updateIcon: (queryClient: QueryClient) => ({
		mutationFn: (args: { profileId: string; selection: ProfileIconSelection }) =>
			rustInvoke('profiles_update_icon', args),
		onSuccess: async (_data: void, args: { profileId: string }) => {
			await invalidateProfileAndDiskQueries(queryClient, args);
			await queryClient.invalidateQueries({
				predicate: (query) =>
					query.queryKey[0] === 'profiles' && query.queryKey[1] === 'binary-file'
			});
		}
	}),

	addMod: (queryClient: QueryClient) => ({
		mutationFn: (args: { profileId: string; modId: string; version: string; file: string }) =>
			rustInvoke('profiles_add_mod', args),
		onSuccess: async (_data: void, args: { profileId: string }) => {
			await invalidateProfileAndDiskQueries(queryClient, args);
		}
	}),

	removeMod: (queryClient: QueryClient) => ({
		mutationFn: (args: { profileId: string; modId: string }) =>
			rustInvoke('profiles_remove_mod', args),
		onSuccess: async (_data: void, args: { profileId: string }) => {
			await invalidateProfileAndDiskQueries(queryClient, args);
		}
	}),

	deleteUnifiedMod: (queryClient: QueryClient) => ({
		mutationFn: async (args: { profileId: string; mod: UnifiedMod }) => {
			const profile = await getProfileById(args.profileId);
			if (!profile) {
				throw new Error(`Profile '${args.profileId}' not found`);
			}

			await rustInvoke('profiles_delete_mod_file', {
				profilePath: profile.path,
				fileName: args.mod.file
			});
			if (args.mod.source === 'managed') {
				await rustInvoke('profiles_remove_mod', {
					profileId: args.profileId,
					modId: args.mod.mod_id
				});
			}
		},
		onSuccess: async (_data: void, args: { profileId: string }) => {
			await invalidateProfileAndDiskQueries(queryClient, args);
		}
	}),

	cleanupMissingMods: (queryClient: QueryClient) => ({
		mutationFn: async (profileId: string) => {
			const profile = await getProfileById(profileId);
			if (!profile) return;
			const diskFiles = await rustInvoke('profiles_get_mod_files', { profilePath: profile.path });
			const diskSet = new Set(diskFiles);
			const missingMods = profile.mods.filter((mod) => mod.file && !diskSet.has(mod.file));
			await Promise.allSettled(
				missingMods.map((mod) =>
					rustInvoke('profiles_remove_mod', { profileId, modId: mod.mod_id })
				)
			);
		},
		onSuccess: async () => {
			queryClient.invalidateQueries({ queryKey: profilesQueryKey });
		}
	}),

	updatePlayTime: (queryClient: QueryClient) => ({
		mutationFn: (args: { profileId: string; durationMs: number }) =>
			rustInvoke('profiles_add_play_time', args),
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: profilesQueryKey });
		}
	}),

	retryBepInExInstall: (queryClient: QueryClient) => ({
		mutationFn: (args: { profileId: string }) => installBepInExForProfile(args.profileId),
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: profilesQueryKey });
		}
	}),

	exportZip: () => ({
		mutationFn: (args: { profileId: string; destination: string }) =>
			rustInvoke('profiles_export_zip', args)
	}),

	importZip: (queryClient: QueryClient) => ({
		mutationFn: (zipPath: string) => rustInvoke('profiles_import_zip', { zipPath }),
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: profilesQueryKey });
		}
	}),

	updateLastLaunched: (queryClient: QueryClient) => ({
		mutationFn: (profileId: string) => rustInvoke('profiles_update_last_launched', { profileId }),
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: profilesQueryKey });
		}
	}),

	installMods: (queryClient: QueryClient) => ({
		mutationFn: (args: InstallArgs) => installModsForProfile(queryClient, args),
		onSuccess: (_data: InstalledModResult[], args: InstallArgs) => {
			void invalidateAfterModInstall(queryClient, args);
		}
	}),

	launchProfile: (queryClient?: QueryClient) => ({
		mutationFn: async (profile: Profile) => {
			if (launchInFlight) {
				throw new Error('A launch is already in progress');
			}
			launchInFlight = true;
			try {
				const settings = await rustInvoke('core_get_settings');
				if (!settings.among_us_path?.trim()) {
					throw new Error('Among Us path not configured');
				}
				if (!settings.allow_multi_instance_launch && gameState.running) {
					throw new Error('An Among Us instance is already running');
				}
				if (settings.game_platform === 'epic') {
					await ensureEpicLogin();
				}

				if (settings.game_platform === 'xbox') {
					await launchXboxProfile(settings, profile, queryClient);
				} else {
					await launchModdedProfile(profile, settings);
				}
				await recordLastLaunched(profile.id);
				await closeWindowAfterLaunch(settings.close_on_launch);
			} finally {
				launchInFlight = false;
			}
		}
	}),

	launchVanilla: (queryClient?: QueryClient) => ({
		mutationFn: async () => {
			if (launchInFlight) {
				throw new Error('A launch is already in progress');
			}
			launchInFlight = true;
			try {
				const settings = await rustInvoke('core_get_settings');
				if (!settings.among_us_path?.trim()) {
					throw new Error('Among Us path not configured');
				}
				if (!settings.allow_multi_instance_launch && gameState.running) {
					throw new Error('An Among Us instance is already running');
				}
				if (settings.game_platform === 'epic') {
					await ensureEpicLogin();
				}

				if (settings.game_platform === 'xbox') {
					await launchXboxVanilla(settings, queryClient);
				} else {
					await launchVanillaGame(settings);
				}

				await closeWindowAfterLaunch(settings.close_on_launch);
			} finally {
				launchInFlight = false;
			}
		}
	})
};

export type CreateProfileAction = ReturnType<typeof profileActions.create>;
export type DeleteProfileAction = ReturnType<typeof profileActions.delete>;
