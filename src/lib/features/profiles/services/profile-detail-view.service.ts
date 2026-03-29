import type { QueryClient } from '@tanstack/svelte-query';

import { pickDefaultVersion } from '$lib/features/mods/components/mod-utils';
import { modQueries } from '$lib/features/mods/queries';
import type { Mod } from '$lib/features/mods/schema';
import type { ProfileModUpdatesMap, UnifiedMod } from '$lib/features/profiles/schema';

export interface ManagedProfileModVersion {
	modId: string;
	installedVersion: string;
}

export async function fetchProfileModUpdates(
	client: QueryClient,
	managedMods: ManagedProfileModVersion[]
): Promise<ProfileModUpdatesMap> {
	const updatesByModId: ProfileModUpdatesMap = {};

	for (const mod of managedMods) {
		updatesByModId[mod.modId] = {
			installedVersion: mod.installedVersion,
			latestVersion: null,
			isOutdated: false,
			status: 'checking'
		};
	}

	const results = await Promise.allSettled(
		managedMods.map(async (mod) => {
			const versions = await client.fetchQuery(modQueries.versions(mod.modId));
			const latestVersion = pickDefaultVersion(versions);
			return { mod, latestVersion };
		})
	);

	for (const result of results) {
		if (result.status === 'rejected') continue;
		const { mod, latestVersion } = result.value;
		updatesByModId[mod.modId] = {
			installedVersion: mod.installedVersion,
			latestVersion: latestVersion ?? null,
			isOutdated: Boolean(latestVersion && latestVersion !== mod.installedVersion),
			status: 'ready'
		};
	}

	for (const mod of managedMods) {
		const entry = updatesByModId[mod.modId];
		if (entry?.status === 'checking') {
			updatesByModId[mod.modId] = {
				installedVersion: mod.installedVersion,
				latestVersion: null,
				isOutdated: false,
				status: 'error'
			};
		}
	}

	return updatesByModId;
}

export function filterProfileMods(unified: UnifiedMod[], modsById: Map<string, Mod>, search: string) {
	const searchLower = search.trim().toLowerCase();
	return unified.filter((mod) => {
		if (!searchLower) return true;
		if (mod.source === 'managed') {
			return modsById.get(mod.mod_id)?.name.toLowerCase().includes(searchLower) ?? false;
		}
		return mod.file.toLowerCase().includes(searchLower);
	});
}

export function paginateProfileMods(mods: UnifiedMod[], pageIndex: number, pageSize = 6) {
	const start = pageIndex * pageSize;
	return mods.slice(start, start + pageSize);
}

export function getProfileModsPagination(total: number, pageIndex: number, pageSize = 6) {
	const totalPages = Math.ceil(total / pageSize);
	const hasNextPage = pageIndex < totalPages - 1;
	return {
		totalPages,
		hasNextPage,
		showPagination: pageIndex > 0 || hasNextPage
	};
}
