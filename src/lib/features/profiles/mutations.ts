import type { QueryClient } from '@tanstack/svelte-query';
import { profileService } from './profile-service';
import { modInstallService } from './mod-install-service';
import type { UnifiedMod } from './schema';

export const profileMutations = {
	create: (queryClient: QueryClient) => ({
		mutationFn: (name: string) => profileService.createProfile(name),
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: ['profiles'] });
		}
	}),

	delete: (queryClient: QueryClient) => ({
		mutationFn: (profileId: string) => profileService.deleteProfile(profileId),
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: ['profiles'] });
		}
	}),

	addMod: (queryClient: QueryClient) => ({
		mutationFn: (args: { profileId: string; modId: string; version: string; file: string }) =>
			profileService.addModToProfile(args.profileId, args.modId, args.version, args.file),
		onSuccess: async (_data: void, args: { profileId: string }) => {
			const profiles = queryClient.getQueryData<{ id: string; path: string }[]>(['profiles']);
			const profile = profiles?.find((p) => p.id === args.profileId);
			queryClient.invalidateQueries({ queryKey: ['profiles'] });
			if (profile?.path) {
				queryClient.invalidateQueries({ queryKey: ['disk-files', profile.path] });
			}
		}
	}),

	removeMod: (queryClient: QueryClient) => ({
		mutationFn: (args: { profileId: string; modId: string }) =>
			profileService.removeModFromProfile(args.profileId, args.modId),
		onSuccess: async (_data: void, args: { profileId: string }) => {
			const profiles = queryClient.getQueryData<{ id: string; path: string }[]>(['profiles']);
			const profile = profiles?.find((p) => p.id === args.profileId);
			queryClient.invalidateQueries({ queryKey: ['profiles'] });
			if (profile?.path) {
				queryClient.invalidateQueries({ queryKey: ['disk-files', profile.path] });
			}
		}
	}),

	deleteUnifiedMod: (queryClient: QueryClient) => ({
		mutationFn: (args: { profileId: string; mod: UnifiedMod }) =>
			profileService.deleteUnifiedMod(args.profileId, args.mod),
		onSuccess: async (_data: void, args: { profileId: string }) => {
			const profiles = queryClient.getQueryData<{ id: string; path: string }[]>(['profiles']);
			const profile = profiles?.find((p) => p.id === args.profileId);
			queryClient.invalidateQueries({ queryKey: ['profiles'] });
			if (profile?.path) {
				queryClient.invalidateQueries({ queryKey: ['disk-files', profile.path] });
			}
		}
	}),

	cleanupMissingMods: (queryClient: QueryClient) => ({
		mutationFn: (profileId: string) => profileService.cleanupMissingMods(profileId),
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: ['profiles'] });
		}
	}),

	updatePlayTime: (queryClient: QueryClient) => ({
		mutationFn: (args: { profileId: string; durationMs: number }) =>
			profileService.addPlayTime(args.profileId, args.durationMs),
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: ['profiles'] });
		}
	}),

	retryBepInExInstall: (queryClient: QueryClient) => ({
		mutationFn: (args: { profileId: string; profilePath: string }) =>
			profileService.retryBepInExInstall(args.profileId, args.profilePath),
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: ['profiles'] });
		}
	}),

	updateLastLaunched: (queryClient: QueryClient) => ({
		mutationFn: (profileId: string) => profileService.updateLastLaunched(profileId),
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: ['profiles'] });
			queryClient.invalidateQueries({ queryKey: ['profiles', 'active'] });
		}
	}),

	installMods: (queryClient: QueryClient) => ({
		mutationFn: async (args: {
			profileId: string;
			profilePath: string;
			mods: Array<{ modId: string; version: string }>;
		}) => {
			const results = await modInstallService.installModsToProfile(args.mods, args.profilePath);
			for (const result of results) {
				await profileService.addModToProfile(
					args.profileId,
					result.modId,
					result.version,
					result.fileName
				);
			}
			return results;
		},
		onSuccess: (
			_data: Array<{ modId: string; version: string; fileName: string }>,
			args: { profileId: string; profilePath: string }
		) => {
			queryClient.invalidateQueries({ queryKey: ['profiles'] });
			queryClient.invalidateQueries({ queryKey: ['disk-files', args.profilePath] });
		}
	})
};

// Type helpers for mutation results
export type CreateProfileMutation = ReturnType<typeof profileMutations.create>;
export type DeleteProfileMutation = ReturnType<typeof profileMutations.delete>;
