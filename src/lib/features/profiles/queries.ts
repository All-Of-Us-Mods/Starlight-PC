import { queryOptions } from '@tanstack/svelte-query';
import { invoke } from '@tauri-apps/api/core';
import type { Profile, UnifiedMod } from './schema';
import {
	profileDiskFilesKey,
	profileLogKey,
	profileUnifiedModsKey,
	profilesActiveQueryKey,
	profilesHasAnyQueryKey,
	profilesQueryKey
} from './profile-keys';

export const profileQueries = {
	dir: () =>
		queryOptions({
			queryKey: ['profiles', 'dir'],
			queryFn: () => invoke<string>('profiles_get_dir')
		}),
	all: () =>
		queryOptions({
			queryKey: profilesQueryKey,
			queryFn: () => invoke<Profile[]>('profiles_list')
		}),
	active: () =>
		queryOptions({
			queryKey: profilesActiveQueryKey,
			queryFn: () => invoke<Profile | null>('profiles_get_active')
		}),
	hasAny: () =>
		queryOptions({
			queryKey: profilesHasAnyQueryKey,
			queryFn: () => invoke<Profile[]>('profiles_list').then((profiles) => profiles.length > 0)
		}),
	diskFiles: (profilePath: string) =>
		queryOptions({
			queryKey: profileDiskFilesKey(profilePath),
			queryFn: () => invoke<string[]>('profiles_get_mod_files', { args: { profilePath } }),
			enabled: !!profilePath
		}),
	unifiedMods: (profileId: string) =>
		queryOptions({
			queryKey: profileUnifiedModsKey(profileId),
			queryFn: () => invoke<UnifiedMod[]>('profiles_get_unified_mods', { args: { profileId } }),
			enabled: !!profileId
		}),
	log: (profilePath: string, fileName = 'LogOutput.log') =>
		queryOptions({
			queryKey: profileLogKey(profilePath, fileName),
			queryFn: () => invoke<string>('profiles_get_log', { args: { profilePath, fileName } }),
			enabled: !!profilePath
		})
};
