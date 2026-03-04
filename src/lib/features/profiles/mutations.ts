import type { QueryClient } from '@tanstack/svelte-query';
import { profileWorkflowService } from './profile-workflow-service';
import type { ProfileIconSelection, UnifiedMod } from './schema';
import { profileDiskFilesKey, profilesActiveQueryKey, profilesQueryKey } from './profile-keys';

type ProfileSummary = { id: string; path: string };
type InstallArgs = {
	profileId: string;
	profilePath: string;
	mods: Array<{ modId: string; version: string }>;
};

function getProfilePathFromCache(queryClient: QueryClient, profileId: string): string | undefined {
	const profiles = queryClient.getQueryData<ProfileSummary[]>(profilesQueryKey);
	return profiles?.find((profile) => profile.id === profileId)?.path;
}

async function invalidateProfileAndDiskQueries(
	queryClient: QueryClient,
	args: { profileId: string; profilePath?: string }
) {
	await queryClient.invalidateQueries({ queryKey: profilesQueryKey });
	const profilePath = args.profilePath ?? getProfilePathFromCache(queryClient, args.profileId);
	if (profilePath) {
		await queryClient.invalidateQueries({ queryKey: profileDiskFilesKey(profilePath) });
	}
}

export const profileMutations = {
	create: (queryClient: QueryClient) => ({
		mutationFn: (name: string) => profileWorkflowService.createProfile(name),
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: profilesQueryKey });
		}
	}),

	delete: (queryClient: QueryClient) => ({
		mutationFn: (profileId: string) => profileWorkflowService.deleteProfile(profileId),
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: profilesQueryKey });
		}
	}),

	rename: (queryClient: QueryClient) => ({
		mutationFn: (args: { profileId: string; newName: string }) =>
			profileWorkflowService.renameProfile(args.profileId, args.newName),
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: profilesQueryKey });
		}
	}),

	updateIcon: (queryClient: QueryClient) => ({
		mutationFn: (args: { profileId: string; selection: ProfileIconSelection }) =>
			profileWorkflowService.updateProfileIcon(args.profileId, args.selection),
		onSuccess: async (_data: void, args: { profileId: string }) => {
			await invalidateProfileAndDiskQueries(queryClient, args);
			await queryClient.invalidateQueries({ queryKey: profilesActiveQueryKey });
		}
	}),

	addMod: (queryClient: QueryClient) => ({
		mutationFn: (args: { profileId: string; modId: string; version: string; file: string }) =>
			profileWorkflowService.addModToProfile(args.profileId, args.modId, args.version, args.file),
		onSuccess: async (_data: void, args: { profileId: string }) => {
			await invalidateProfileAndDiskQueries(queryClient, args);
		}
	}),

	removeMod: (queryClient: QueryClient) => ({
		mutationFn: (args: { profileId: string; modId: string }) =>
			profileWorkflowService.removeModFromProfile(args.profileId, args.modId),
		onSuccess: async (_data: void, args: { profileId: string }) => {
			await invalidateProfileAndDiskQueries(queryClient, args);
		}
	}),

	deleteUnifiedMod: (queryClient: QueryClient) => ({
		mutationFn: (args: { profileId: string; mod: UnifiedMod }) =>
			profileWorkflowService.deleteUnifiedMod(args.profileId, args.mod),
		onSuccess: async (_data: void, args: { profileId: string }) => {
			await invalidateProfileAndDiskQueries(queryClient, args);
		}
	}),

	cleanupMissingMods: (queryClient: QueryClient) => ({
		mutationFn: (profileId: string) => profileWorkflowService.cleanupMissingMods(profileId),
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: profilesQueryKey });
		}
	}),

	updatePlayTime: (queryClient: QueryClient) => ({
		mutationFn: (args: { profileId: string; durationMs: number }) =>
			profileWorkflowService.addPlayTime(args.profileId, args.durationMs),
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: profilesQueryKey });
		}
	}),

	retryBepInExInstall: (queryClient: QueryClient) => ({
		mutationFn: (args: { profileId: string; profilePath: string }) =>
			profileWorkflowService.retryBepInExInstall(args.profileId, args.profilePath),
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: profilesQueryKey });
		}
	}),

	exportZip: () => ({
		mutationFn: (args: { profileId: string; destination: string }) =>
			profileWorkflowService.exportProfileZip(args.profileId, args.destination)
	}),

	importZip: (queryClient: QueryClient) => ({
		mutationFn: (zipPath: string) => profileWorkflowService.importProfileZip(zipPath),
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: profilesQueryKey });
			queryClient.invalidateQueries({ queryKey: profilesActiveQueryKey });
		}
	}),

	updateLastLaunched: (queryClient: QueryClient) => ({
		mutationFn: (profileId: string) => profileWorkflowService.updateLastLaunched(profileId),
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: profilesQueryKey });
			queryClient.invalidateQueries({ queryKey: profilesActiveQueryKey });
		}
	}),

	installMods: (queryClient: QueryClient) => ({
		mutationFn: (args: InstallArgs) =>
			profileWorkflowService.installMods(args.profileId, args.profilePath, args.mods),
		onSuccess: (
			_data: Array<{ mod_id: string; version: string; file_name: string }>,
			args: InstallArgs
		) => {
			void invalidateProfileAndDiskQueries(queryClient, args);
		}
	})
};

// Type helpers for mutation results
export type CreateProfileMutation = ReturnType<typeof profileMutations.create>;
export type DeleteProfileMutation = ReturnType<typeof profileMutations.delete>;
