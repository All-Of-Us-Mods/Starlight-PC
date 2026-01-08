import { createMutation, useQueryClient } from '@tanstack/svelte-query';
import { profileService } from './profile-service';
import type { Profile, UnifiedMod } from './schema';

export function useCreateProfile() {
	const queryClient = useQueryClient();
	return createMutation<Profile, Error, string>(() => ({
		mutationFn: (name) => profileService.createProfile(name),
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: ['profiles'] });
		}
	}));
}

export function useDeleteProfile() {
	const queryClient = useQueryClient();
	return createMutation<void, Error, string>(() => ({
		mutationFn: (profileId) => profileService.deleteProfile(profileId),
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: ['profiles'] });
		}
	}));
}

export function useAddModToProfile() {
	const queryClient = useQueryClient();
	return createMutation<
		void,
		Error,
		{ profileId: string; modId: string; version: string; file: string }
	>(() => ({
		mutationFn: (args) =>
			profileService.addModToProfile(args.profileId, args.modId, args.version, args.file),
		onSuccess: (_data, args) => {
			queryClient.invalidateQueries({ queryKey: ['profiles'] });
			queryClient.invalidateQueries({ queryKey: ['unified-mods', args.profileId] });
		}
	}));
}

export function useRemoveModFromProfile() {
	const queryClient = useQueryClient();
	return createMutation<void, Error, { profileId: string; modId: string }>(() => ({
		mutationFn: (args) => profileService.removeModFromProfile(args.profileId, args.modId),
		onSuccess: (_data, args) => {
			queryClient.invalidateQueries({ queryKey: ['profiles'] });
			queryClient.invalidateQueries({ queryKey: ['unified-mods', args.profileId] });
		}
	}));
}

export function useDeleteUnifiedMod() {
	const queryClient = useQueryClient();
	return createMutation<void, Error, { profileId: string; mod: UnifiedMod }>(() => ({
		mutationFn: (args) => profileService.deleteUnifiedMod(args.profileId, args.mod),
		onSuccess: (_data, args) => {
			queryClient.invalidateQueries({ queryKey: ['profiles'] });
			queryClient.invalidateQueries({ queryKey: ['unified-mods', args.profileId] });
		}
	}));
}

export function useUpdatePlayTime() {
	const queryClient = useQueryClient();
	return createMutation<void, Error, { profileId: string; durationMs: number }>(() => ({
		mutationFn: (args) => profileService.addPlayTime(args.profileId, args.durationMs),
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: ['profiles'] });
		}
	}));
}

export function useRetryBepInExInstall() {
	const queryClient = useQueryClient();
	return createMutation<void, Error, { profileId: string; profilePath: string }>(() => ({
		mutationFn: (args) => profileService.retryBepInExInstall(args.profileId, args.profilePath),
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: ['profiles'] });
		}
	}));
}

export function useUpdateLastLaunched() {
	const queryClient = useQueryClient();
	return createMutation<void, Error, string>(() => ({
		mutationFn: (profileId) => profileService.updateLastLaunched(profileId),
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: ['profiles'] });
			queryClient.invalidateQueries({ queryKey: ['profiles', 'active'] });
		}
	}));
}
