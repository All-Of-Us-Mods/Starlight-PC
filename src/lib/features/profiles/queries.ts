import { queryOptions } from '@tanstack/svelte-query';
import { profileService } from './profile-service';

export const profileQueries = {
	all: () =>
		queryOptions({
			queryKey: ['profiles'] as const,
			queryFn: () => profileService.getProfiles(),
			staleTime: Infinity
		}),
	active: () =>
		queryOptions({
			queryKey: ['profiles', 'active'] as const,
			queryFn: () => profileService.getActiveProfile(),
			staleTime: Infinity
		}),
	hasAny: () =>
		queryOptions({
			queryKey: ['profiles', 'hasAny'] as const,
			queryFn: () => profileService.getProfiles().then((profiles) => profiles.length > 0),
			staleTime: Infinity
		})
};
