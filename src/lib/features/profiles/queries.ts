import { queryOptions, type QueryClient } from '@tanstack/svelte-query';
import { rustQueryOptions } from '$lib/infra/rust/query';
import { rustInvoke } from '$lib/infra/rust/invoke';
import {
	profileDiskFilesKey,
	profileLogKey,
	profileUnifiedModsKey,
	profilesQueryKey
} from './profile-keys';
import type { Profile, UnifiedMod } from './schema';

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
	diskFiles: (profilePath: string) =>
		rustQueryOptions({
			queryKey: profileDiskFilesKey(profilePath),
			command: 'profiles_get_mod_files',
			args: { profilePath },
			enabled: !!profilePath
		}),
	unifiedMods: (profileId: string, queryClient?: QueryClient) =>
		queryOptions({
			queryKey: profileUnifiedModsKey(profileId),
			queryFn: async () => {
				const profiles: Profile[] = queryClient
					? await queryClient.fetchQuery(profileQueries.all())
					: await rustInvoke('profiles_list');
				const profile = profiles.find((entry) => entry.id === profileId);
				if (!profile) return [];

				const diskFiles = queryClient
					? await queryClient.fetchQuery(profileQueries.diskFiles(profile.path))
					: await rustInvoke('profiles_get_mod_files', { profilePath: profile.path });
				const diskSet = new Set(diskFiles);
				const managedFiles = new Set<string>();
				const unified: UnifiedMod[] = [];

				for (const mod of profile.mods) {
					if (!mod.file) continue;
					managedFiles.add(mod.file);
					if (diskSet.has(mod.file)) {
						unified.push({
							source: 'managed',
							mod_id: mod.mod_id,
							version: mod.version,
							file: mod.file
						});
					}
				}

				for (const file of diskFiles) {
					if (!managedFiles.has(file)) {
						unified.push({ source: 'custom', file });
					}
				}

				return unified;
			},
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
