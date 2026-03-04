import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { QueryClient } from '@tanstack/svelte-query';
import { gameState } from './game-state.svelte';
import type { BepInExProgress, Profile, ProfileIconSelection, UnifiedMod } from './schema';
import { profileDiskFilesKey, profilesActiveQueryKey, profilesQueryKey } from './profile-keys';

type ProfileSummary = { id: string; path: string };
type InstallArgs = {
	profileId: string;
	profilePath: string;
	mods: Array<{ modId: string; version: string }>;
};

async function installBepInEx(profileId: string, profilePath: string) {
	let unlisten: (() => void) | undefined;
	try {
		unlisten = await listen<BepInExProgress>('bepinex-progress', (event) => {
			gameState.setBepInExProgress(profileId, event.payload);
		});
		await invoke<void>('profiles_install_bepinex', { args: { profileId, profilePath } });
	} catch (error) {
		const message = error instanceof Error ? error.message : 'Unknown error';
		gameState.setBepInExError(profileId, message);
		throw error;
	} finally {
		unlisten?.();
		gameState.clearBepInExProgress(profileId);
	}
}

function getProfilePathFromCache(queryClient: QueryClient, profileId: string): string | undefined {
	const profiles = queryClient.getQueryData<ProfileSummary[]>(profilesQueryKey);
	return profiles?.find((profile) => profile.id === profileId)?.path;
}

async function invalidateProfileAndDiskQueries(
	queryClient: QueryClient,
	args: { profileId: string; profilePath?: string }
) {
	await queryClient.invalidateQueries({ queryKey: profilesQueryKey });
	const profilePath = args.profilePath ?? getProfilePathFromCache(queryClient, args.profileId);
	if (profilePath) {
		await queryClient.invalidateQueries({ queryKey: profileDiskFilesKey(profilePath) });
	}
}

export const profileMutations = {
	create: (queryClient: QueryClient) => ({
		mutationFn: async (name: string) => {
			const profile = await invoke<Profile>('profiles_create', { args: { name } });
			void installBepInEx(profile.id, profile.path).finally(() => {
				void queryClient.invalidateQueries({ queryKey: profilesQueryKey });
			});
			return profile;
		},
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: profilesQueryKey });
		}
	}),

	delete: (queryClient: QueryClient) => ({
		mutationFn: (profileId: string) => invoke<void>('profiles_delete', { args: { profileId } }),
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: profilesQueryKey });
		}
	}),

	rename: (queryClient: QueryClient) => ({
		mutationFn: (args: { profileId: string; newName: string }) =>
			invoke<void>('profiles_rename', { args }),
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: profilesQueryKey });
		}
	}),

	updateIcon: (queryClient: QueryClient) => ({
		mutationFn: (args: { profileId: string; selection: ProfileIconSelection }) =>
			invoke<void>('profiles_update_icon', { args }),
		onSuccess: async (_data: void, args: { profileId: string }) => {
			await invalidateProfileAndDiskQueries(queryClient, args);
			await queryClient.invalidateQueries({ queryKey: profilesActiveQueryKey });
		}
	}),

	addMod: (queryClient: QueryClient) => ({
		mutationFn: (args: { profileId: string; modId: string; version: string; file: string }) =>
			invoke<void>('profiles_add_mod', { args }),
		onSuccess: async (_data: void, args: { profileId: string }) => {
			await invalidateProfileAndDiskQueries(queryClient, args);
		}
	}),

	removeMod: (queryClient: QueryClient) => ({
		mutationFn: (args: { profileId: string; modId: string }) =>
			invoke<void>('profiles_remove_mod', { args }),
		onSuccess: async (_data: void, args: { profileId: string }) => {
			await invalidateProfileAndDiskQueries(queryClient, args);
		}
	}),

	deleteUnifiedMod: (queryClient: QueryClient) => ({
		mutationFn: (args: { profileId: string; mod: UnifiedMod }) =>
			invoke<void>('profiles_delete_unified_mod', {
				args: { profileId: args.profileId, modEntry: args.mod }
			}),
		onSuccess: async (_data: void, args: { profileId: string }) => {
			await invalidateProfileAndDiskQueries(queryClient, args);
		}
	}),

	cleanupMissingMods: (queryClient: QueryClient) => ({
		mutationFn: (profileId: string) =>
			invoke<void>('profiles_cleanup_missing_mods', { args: { profileId } }),
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: profilesQueryKey });
		}
	}),

	updatePlayTime: (queryClient: QueryClient) => ({
		mutationFn: (args: { profileId: string; durationMs: number }) =>
			invoke<void>('profiles_add_play_time', { args }),
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: profilesQueryKey });
		}
	}),

	retryBepInExInstall: (queryClient: QueryClient) => ({
		mutationFn: (args: { profileId: string; profilePath: string }) =>
			installBepInEx(args.profileId, args.profilePath),
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: profilesQueryKey });
		}
	}),

	exportZip: () => ({
		mutationFn: (args: { profileId: string; destination: string }) =>
			invoke<void>('profiles_export_zip', { args })
	}),

	importZip: (queryClient: QueryClient) => ({
		mutationFn: (zipPath: string) => invoke<Profile>('profiles_import_zip', { args: { zipPath } }),
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: profilesQueryKey });
			queryClient.invalidateQueries({ queryKey: profilesActiveQueryKey });
		}
	}),

	updateLastLaunched: (queryClient: QueryClient) => ({
		mutationFn: (profileId: string) =>
			invoke<void>('profiles_update_last_launched', { args: { profileId } }),
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: profilesQueryKey });
			queryClient.invalidateQueries({ queryKey: profilesActiveQueryKey });
		}
	}),

	installMods: (queryClient: QueryClient) => ({
		mutationFn: (args: InstallArgs) =>
			invoke<Array<{ mod_id: string; version: string; file_name: string }>>(
				'modding_install_profile_mods',
				{ args }
			),
		onSuccess: (
			_data: Array<{ mod_id: string; version: string; file_name: string }>,
			args: InstallArgs
		) => {
			void invalidateProfileAndDiskQueries(queryClient, args);
		}
	}),

	launchProfile: () => ({
		mutationFn: async (profile: Profile) => {
			const result = await invoke<{ close_on_launch: boolean }>('game_launch_profile', {
				args: { profileId: profile.id, profilePath: profile.path }
			});
			if (result.close_on_launch) {
				const { getCurrentWindow } = await import('@tauri-apps/api/window');
				await getCurrentWindow().close();
			}
		}
	}),

	launchVanilla: () => ({
		mutationFn: async () => {
			const result = await invoke<{ close_on_launch: boolean }>('game_launch_vanilla_workflow');
			if (result.close_on_launch) {
				const { getCurrentWindow } = await import('@tauri-apps/api/window');
				await getCurrentWindow().close();
			}
		}
	})
};

export type CreateProfileMutation = ReturnType<typeof profileMutations.create>;
export type DeleteProfileMutation = ReturnType<typeof profileMutations.delete>;
