import { queryOptions } from '@tanstack/svelte-query';
import { type } from 'arktype';
import { apiFetch } from '$lib/api/client';
import { ModVersion } from '../mods/schema';
import { profileService } from './profile-service';
import { settingsService } from '../settings/settings-service';

const ModVersionsArray = type(ModVersion.array());

export const modQueries = {
	versions: (modId: string) =>
		queryOptions({
			queryKey: ['mods', 'versions', modId] as const,
			queryFn: () => apiFetch(`/api/v2/mods/${modId}/versions`, ModVersionsArray),
			staleTime: 1000 * 60 * 5
		})
};

export const profileQueries = {
	all: () =>
		queryOptions({
			queryKey: ['profiles'] as const,
			queryFn: () => profileService.getProfiles()
		}),
	active: () =>
		queryOptions({
			queryKey: ['profiles', 'active'] as const,
			queryFn: () => profileService.getActiveProfile()
		})
};

export const settingsQueries = {
	get: () =>
		queryOptions({
			queryKey: ['settings'] as const,
			queryFn: () => settingsService.getSettings()
		})
};
