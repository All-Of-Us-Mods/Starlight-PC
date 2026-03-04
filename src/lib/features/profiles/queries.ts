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
import type { UnifiedMod } from './schema';

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
		queryOptions({
			queryKey: profileUnifiedModsKey(profileId),
			queryFn: async () => {
				const profiles = await rustInvoke('profiles_list');
				const profile = profiles.find((entry) => entry.id === profileId);
				if (!profile) return [];

				const diskFiles = await rustInvoke('profiles_get_mod_files', { profilePath: profile.path });
				const diskSet = new Set(diskFiles);
				const managedFiles = new Set<string>();
				const unified: UnifiedMod[] = [];
				const missingManaged = new Set<string>();

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
					} else {
						missingManaged.add(mod.mod_id);
					}
				}

				for (const file of diskFiles) {
					if (!managedFiles.has(file)) {
						unified.push({ source: 'custom', file });
					}
				}

				if (missingManaged.size > 0) {
					await Promise.all(
						Array.from(missingManaged).map((modId) =>
							rustInvoke('profiles_remove_mod', { profileId, modId }).catch(() => undefined)
						)
					);
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
