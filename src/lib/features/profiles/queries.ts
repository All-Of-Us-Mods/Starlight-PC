import { queryOptions } from '@tanstack/svelte-query';
import { rustQueryOptions } from '$lib/infra/rust/query';
import { rustInvoke } from '$lib/infra/rust/invoke';
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
		rustQueryOptions({
			queryKey: ['profiles', 'dir'],
			command: 'profiles_get_dir'
		}),
	all: () =>
		rustQueryOptions({
			queryKey: profilesQueryKey,
			command: 'profiles_list'
		}),
	active: () =>
		rustQueryOptions({
			queryKey: profilesActiveQueryKey,
			command: 'profiles_get_active'
		}),
	hasAny: () =>
		queryOptions({
			queryKey: profilesHasAnyQueryKey,
			queryFn: () => rustInvoke('profiles_list').then((profiles) => profiles.length > 0)
		}),
	diskFiles: (profilePath: string) =>
		rustQueryOptions({
			queryKey: profileDiskFilesKey(profilePath),
			command: 'profiles_get_mod_files',
			args: { profilePath },
			enabled: !!profilePath
		}),
	unifiedMods: (profileId: string) =>
		rustQueryOptions({
			queryKey: profileUnifiedModsKey(profileId),
			command: 'profiles_get_unified_mods',
			args: { profileId },
			enabled: !!profileId
		}),
	binaryFile: (path: string) =>
		rustQueryOptions({
			queryKey: ['profiles', 'binary-file', path],
			command: 'profiles_read_binary_file',
			args: { path },
			enabled: !!path
		}),
	log: (profilePath: string, fileName = 'LogOutput.log') =>
		rustQueryOptions({
			queryKey: profileLogKey(profilePath, fileName),
			command: 'profiles_get_log',
			args: { profilePath, fileName },
			enabled: !!profilePath
		})
};
