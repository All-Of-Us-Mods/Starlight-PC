import type { QueryClient } from '@tanstack/svelte-query';
import { modQueries } from '$lib/features/mods/queries';
import { pickDefaultVersion } from '$lib/features/mods/ui/mod-utils';

export interface ManagedProfileModVersion {
	modId: string;
	installedVersion: string;
}

export type ProfileModUpdateStatus = {
	installedVersion: string;
	latestVersion: string | null;
	isOutdated: boolean;
	status: 'checking' | 'ready' | 'error';
};

export type ProfileModUpdatesMap = Record<string, ProfileModUpdateStatus>;

export async function fetchProfileModUpdates(
	queryClient: QueryClient,
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
			const versions = await queryClient.fetchQuery(modQueries.versions(mod.modId));
			const latestVersion = pickDefaultVersion(versions);
			return { mod, latestVersion };
		})
	);

	for (const result of results) {
		if (result.status === 'rejected') {
			continue;
		}

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
