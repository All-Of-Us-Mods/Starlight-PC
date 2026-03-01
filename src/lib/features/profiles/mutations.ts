import type { QueryClient } from '@tanstack/svelte-query';
import { profileWorkflowService } from './profile-workflow-service';
import { modInstallService } from './mod-install-service';
import type { ProfileIconSelection, UnifiedMod } from './schema';
import { profileDiskFilesKey, profilesActiveQueryKey, profilesQueryKey } from './profile-keys';
import { error as logError, warn } from '@tauri-apps/plugin-log';

type ProfileSummary = { id: string; path: string };
type InstallArgs = {
	profileId: string;
	profilePath: string;
	mods: Array<{ modId: string; version: string }>;
};
type InstalledMod = { modId: string; version: string; fileName: string };

function getProfilePathFromCache(queryClient: QueryClient, profileId: string): string | undefined {
	const profiles = queryClient.getQueryData<ProfileSummary[]>(profilesQueryKey);
	return profiles?.find((profile) => profile.id === profileId)?.path;
}

function invalidateProfileAndDiskQueries(
	queryClient: QueryClient,
	args: { profileId: string; profilePath?: string }
) {
	queryClient.invalidateQueries({ queryKey: profilesQueryKey });
	const profilePath = args.profilePath ?? getProfilePathFromCache(queryClient, args.profileId);
	if (profilePath) {
		queryClient.invalidateQueries({ queryKey: profileDiskFilesKey(profilePath) });
	}
}

async function rollbackInstalledMods(
	args: InstallArgs,
	installed: InstalledMod[],
	persisted: InstalledMod[],
	previousByModId: Map<string, { version: string; file: string | undefined } | undefined>
) {
	for (const mod of persisted.reverse()) {
		const previous = previousByModId.get(mod.modId);
		try {
			if (previous?.file) {
				await profileWorkflowService.addModToProfile(
					args.profileId,
					mod.modId,
					previous.version,
					previous.file
				);
			} else {
				await profileWorkflowService.removeModFromProfile(args.profileId, mod.modId);
			}
		} catch (error) {
			warn(`Failed to rollback metadata for mod "${mod.modId}": ${error}`);
		}
	}

	for (const mod of installed.reverse()) {
		try {
			await profileWorkflowService.deleteModFile(args.profilePath, mod.fileName);
		} catch (error) {
			warn(`Failed to rollback mod file "${mod.fileName}": ${error}`);
		}
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
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: profilesQueryKey });
		}
	}),

	addMod: (queryClient: QueryClient) => ({
		mutationFn: (args: { profileId: string; modId: string; version: string; file: string }) =>
			profileWorkflowService.addModToProfile(args.profileId, args.modId, args.version, args.file),
		onSuccess: async (_data: void, args: { profileId: string }) => {
			invalidateProfileAndDiskQueries(queryClient, args);
		}
	}),

	removeMod: (queryClient: QueryClient) => ({
		mutationFn: (args: { profileId: string; modId: string }) =>
			profileWorkflowService.removeModFromProfile(args.profileId, args.modId),
		onSuccess: async (_data: void, args: { profileId: string }) => {
			invalidateProfileAndDiskQueries(queryClient, args);
		}
	}),

	deleteUnifiedMod: (queryClient: QueryClient) => ({
		mutationFn: (args: { profileId: string; mod: UnifiedMod }) =>
			profileWorkflowService.deleteUnifiedMod(args.profileId, args.mod),
		onSuccess: async (_data: void, args: { profileId: string }) => {
			invalidateProfileAndDiskQueries(queryClient, args);
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
		mutationFn: async (args: InstallArgs) => {
			const profile = await profileWorkflowService.getProfileById(args.profileId);
			if (!profile) throw new Error(`Profile '${args.profileId}' not found`);

			const previousByModId = new Map<
				string,
				{ version: string; file: string | undefined } | undefined
			>();
			for (const mod of args.mods) {
				const previous = profile.mods.find((profileMod) => profileMod.mod_id === mod.modId);
				previousByModId.set(
					mod.modId,
					previous ? { version: previous.version, file: previous.file } : undefined
				);
			}

			const installed: InstalledMod[] = [];
			const persisted: InstalledMod[] = [];
			try {
				for (const mod of args.mods) {
					const fileName = await modInstallService.installModToProfile(
						mod.modId,
						mod.version,
						args.profilePath
					);
					installed.push({ modId: mod.modId, version: mod.version, fileName });
				}

				for (const mod of installed) {
					await profileWorkflowService.addModToProfile(
						args.profileId,
						mod.modId,
						mod.version,
						mod.fileName
					);
					persisted.push(mod);
				}

				for (const mod of persisted) {
					const previous = previousByModId.get(mod.modId);
					if (!previous?.file || previous.file === mod.fileName) continue;
					try {
						await profileWorkflowService.deleteModFile(args.profilePath, previous.file);
					} catch (error) {
						warn(
							`Failed to remove replaced mod file "${previous.file}" for mod "${mod.modId}": ${error}`
						);
					}
				}

				return installed;
			} catch (error) {
				logError(`Failed to install mods for profile "${args.profileId}": ${error}`);
				await rollbackInstalledMods(args, installed, persisted, previousByModId);
				throw error;
			}
		},
		onSuccess: (
			_data: Array<{ modId: string; version: string; fileName: string }>,
			args: InstallArgs
		) => {
			invalidateProfileAndDiskQueries(queryClient, args);
		}
	})
};

// Type helpers for mutation results
export type CreateProfileMutation = ReturnType<typeof profileMutations.create>;
export type DeleteProfileMutation = ReturnType<typeof profileMutations.delete>;
