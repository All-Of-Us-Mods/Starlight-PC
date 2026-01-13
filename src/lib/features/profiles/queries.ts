import { queryOptions } from '@tanstack/svelte-query';
import { profileService } from './profile-service';

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
		}),
	hasAny: () =>
		queryOptions({
			queryKey: ['profiles', 'hasAny'] as const,
			queryFn: () => profileService.getProfiles().then((profiles) => profiles.length > 0)
		}),
	unifiedMods: (profileId: string) =>
		queryOptions({
			queryKey: ['unified-mods', profileId] as const,
			queryFn: () => profileService.getUnifiedMods(profileId),
			enabled: !!profileId
		})
};
