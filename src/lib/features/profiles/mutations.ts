import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { join } from '@tauri-apps/api/path';
import { exists } from '@tauri-apps/plugin-fs';
import type { QueryClient } from '@tanstack/svelte-query';
import { gameState } from './game-state.svelte';
import type {
	BepInExProgress,
	ModDownloadProgress,
	Profile,
	ProfileIconSelection,
	UnifiedMod
} from './schema';
import { profileDiskFilesKey, profilesQueryKey } from './profile-keys';
import { rustInvoke } from '$lib/infra/rust/invoke';
import type { AppSettings } from '$lib/features/settings/schema';
import { settingsQueryKey } from '$lib/features/settings/settings-keys';
import { modQueries } from '$lib/features/mods/queries';
import type { ModVersionInfo } from '$lib/features/mods/schema';
import { resolveApiUrl } from '$lib/api/client';
import { epicService } from '$lib/features/settings/epic-service';

type ProfileSummary = { id: string; path: string };
type InstallArgs = {
	profileId: string;
	mods: Array<{ modId: string; version: string }>;
};

type PreviousModState = Map<string, { version: string; file?: string } | null>;

type InstalledModResult = { mod_id: string; version: string; file_name: string };

type DownloadTarget = {
	url: string;
	fileName: string;
	checksum: string;
};
let launchInFlight = false;
const bepinexInstallInFlight = new Set<string>();
const modsInstallInFlight = new Set<string>();

function resolveDownloadTarget(
	modId: string,
	version: string,
	versionInfo: ModVersionInfo,
	platform: AppSettings['game_platform']
): DownloadTarget {
	const legacyPath = `/api/v2/mods/${modId}/versions/${version}/file`;
	const defaultUrl = versionInfo.download_url ?? legacyPath;
	const fallback: DownloadTarget = {
		url: resolveApiUrl(defaultUrl),
		fileName: versionInfo.file_name,
		checksum: versionInfo.checksum
	};

	const platforms = versionInfo.platforms ?? [];
	if (platforms.length === 0) return fallback;

	const architectureFallbacks = platform === 'epic' ? ['x64', 'x86'] : ['x86'];
	for (const arch of architectureFallbacks) {
		const entry = platforms.find(
			(candidate) => candidate.platform === 'windows' && candidate.architecture === arch
		);
		if (!entry) continue;
		const downloadUrl = entry.download_url ?? `${legacyPath}?platform=windows&arch=${arch}`;
		return {
			url: resolveApiUrl(downloadUrl),
			fileName: entry.file_name ?? versionInfo.file_name,
			checksum: entry.checksum ?? versionInfo.checksum
		};
	}

	return fallback;
}

async function getProfileById(profileId: string): Promise<Profile | null> {
	return rustInvoke('profiles_get_by_id', { id: profileId });
}

async function rollbackInstalledMods(
	profileId: string,
	profilePath: string,
	installed: InstalledModResult[],
	persisted: InstalledModResult[],
	previousByModId: PreviousModState
) {
	await Promise.all(
		persisted.toReversed().map(async (item) => {
			const previous = previousByModId.get(item.mod_id);
			if (previous?.file) {
				await rustInvoke('profiles_add_mod', {
					profileId,
					modId: item.mod_id,
					version: previous.version,
					file: previous.file
				}).catch((error) => {
					console.warn('[rollback] Failed to restore mod metadata', {
						profileId,
						modId: item.mod_id,
						error
					});
				});
				return;
			}
			await rustInvoke('profiles_remove_mod', {
				profileId,
				modId: item.mod_id
			}).catch((error) => {
				console.warn('[rollback] Failed to remove rolled-back mod metadata', {
					profileId,
					modId: item.mod_id,
					error
				});
			});
		})
	);

	await Promise.all(
		installed.toReversed().map((item) =>
			rustInvoke('profiles_delete_mod_file', { profilePath, fileName: item.file_name }).catch(
				(error) => {
					console.warn('[rollback] Failed to delete rolled-back mod file', {
						profilePath,
						fileName: item.file_name,
						error
					});
				}
			)
		)
	);
}

async function installBepInEx(profileId: string) {
	if (bepinexInstallInFlight.has(profileId)) {
		throw new Error('BepInEx install already in progress for this profile');
	}
	bepinexInstallInFlight.add(profileId);

	let unlisten: (() => void) | undefined;
	let succeeded = false;
	try {
		unlisten = await listen<BepInExProgress>('bepinex-progress', (event) => {
			gameState.setBepInExProgress(profileId, event.payload);
		});
		await rustInvoke('profiles_install_bepinex', { profileId });
		succeeded = true;
	} catch (error) {
		const message = error instanceof Error ? error.message : 'Unknown error';
		gameState.setBepInExError(profileId, message);
		throw error;
	} finally {
		bepinexInstallInFlight.delete(profileId);
		unlisten?.();
		if (succeeded) {
			gameState.clearBepInExProgress(profileId);
		}
	}
}

async function assertPathExists(path: string, message: string) {
	if (!(await exists(path))) {
		throw new Error(message);
	}
}

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
		mutationFn: async (name: string) => {
			const profile = await rustInvoke('profiles_create', { name });
			void installBepInEx(profile.id)
				.catch((error) => {
					console.error('[profiles] Background BepInEx install failed', error);
				})
				.finally(() => {
					void queryClient.invalidateQueries({ queryKey: profilesQueryKey });
				});
			return profile;
		},
		onSuccess: (created: Profile) => {
			queryClient.setQueryData<Profile[] | undefined>(profilesQueryKey, (current) => {
				if (!current) return [created];
				const hasProfile = current.some((profile) => profile.id === created.id);
				return hasProfile
					? current.map((profile) => (profile.id === created.id ? created : profile))
					: [...current, created];
			});
		}
	}),

	delete: (queryClient: QueryClient) => ({
		mutationFn: (profileId: string) => rustInvoke('profiles_delete', { profileId }),
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: profilesQueryKey });
		}
	}),

	rename: (queryClient: QueryClient) => ({
		mutationFn: (args: { profileId: string; newName: string }) =>
			rustInvoke('profiles_rename', args),
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: profilesQueryKey });
		}
	}),

	updateIcon: (queryClient: QueryClient) => ({
		mutationFn: (args: { profileId: string; selection: ProfileIconSelection }) =>
			rustInvoke('profiles_update_icon', args),
		onSuccess: async (_data: void, args: { profileId: string }) => {
			await invalidateProfileAndDiskQueries(queryClient, args);
			await queryClient.invalidateQueries({
				predicate: (query) =>
					query.queryKey[0] === 'profiles' && query.queryKey[1] === 'binary-file'
			});
		}
	}),

	addMod: (queryClient: QueryClient) => ({
		mutationFn: (args: { profileId: string; modId: string; version: string; file: string }) =>
			rustInvoke('profiles_add_mod', args),
		onSuccess: async (_data: void, args: { profileId: string }) => {
			await invalidateProfileAndDiskQueries(queryClient, args);
		}
	}),

	removeMod: (queryClient: QueryClient) => ({
		mutationFn: (args: { profileId: string; modId: string }) =>
			rustInvoke('profiles_remove_mod', args),
		onSuccess: async (_data: void, args: { profileId: string }) => {
			await invalidateProfileAndDiskQueries(queryClient, args);
		}
	}),

	deleteUnifiedMod: (queryClient: QueryClient) => ({
		mutationFn: async (args: { profileId: string; mod: UnifiedMod }) => {
			const profile = await getProfileById(args.profileId);
			if (!profile) {
				throw new Error(`Profile '${args.profileId}' not found`);
			}

			await rustInvoke('profiles_delete_mod_file', {
				profilePath: profile.path,
				fileName: args.mod.file
			});
			if (args.mod.source === 'managed') {
				await rustInvoke('profiles_remove_mod', {
					profileId: args.profileId,
					modId: args.mod.mod_id
				});
			}
		},
		onSuccess: async (_data: void, args: { profileId: string }) => {
			await invalidateProfileAndDiskQueries(queryClient, args);
		}
	}),

	cleanupMissingMods: (queryClient: QueryClient) => ({
		mutationFn: async (profileId: string) => {
			const profile = await getProfileById(profileId);
			if (!profile) return;
			const diskFiles = await rustInvoke('profiles_get_mod_files', { profilePath: profile.path });
			const diskSet = new Set(diskFiles);
			const missingMods = profile.mods.filter((mod) => mod.file && !diskSet.has(mod.file));
			await Promise.allSettled(
				missingMods.map((mod) =>
					rustInvoke('profiles_remove_mod', { profileId, modId: mod.mod_id })
				)
			);
		},
		onSuccess: async () => {
			queryClient.invalidateQueries({ queryKey: profilesQueryKey });
		}
	}),

	updatePlayTime: (queryClient: QueryClient) => ({
		mutationFn: (args: { profileId: string; durationMs: number }) =>
			rustInvoke('profiles_add_play_time', args),
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: profilesQueryKey });
		}
	}),

	retryBepInExInstall: (queryClient: QueryClient) => ({
		mutationFn: (args: { profileId: string }) => installBepInEx(args.profileId),
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: profilesQueryKey });
		}
	}),

	exportZip: () => ({
		mutationFn: (args: { profileId: string; destination: string }) =>
			rustInvoke('profiles_export_zip', args)
	}),

	importZip: (queryClient: QueryClient) => ({
		mutationFn: (zipPath: string) => rustInvoke('profiles_import_zip', { zipPath }),
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: profilesQueryKey });
		}
	}),

	updateLastLaunched: (queryClient: QueryClient) => ({
		mutationFn: (profileId: string) => rustInvoke('profiles_update_last_launched', { profileId }),
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: profilesQueryKey });
		}
	}),

	installMods: (queryClient: QueryClient) => ({
		mutationFn: async (args: InstallArgs) => {
			if (modsInstallInFlight.has(args.profileId)) {
				throw new Error('An install is already in progress for this profile');
			}
			modsInstallInFlight.add(args.profileId);
			let unlistenModDownload: UnlistenFn | undefined;
			let failed = false;
			const installed: InstalledModResult[] = [];

			try {
				const settings = await rustInvoke('core_get_settings');
				const profile = await getProfileById(args.profileId);
				if (!profile) {
					throw new Error(`Profile '${args.profileId}' not found`);
				}
				unlistenModDownload = await listen<ModDownloadProgress>(
					'mod-download-progress',
					(event) => {
						gameState.setModDownloadProgress(event.payload.mod_id, event.payload);
					}
				);

				const previousByModId: PreviousModState = new Map();
				for (const item of args.mods) {
					const previous = profile.mods.find((entry) => entry.mod_id === item.modId);
					previousByModId.set(
						item.modId,
						previous ? { version: previous.version, file: previous.file ?? undefined } : null
					);
				}

				const persisted: InstalledModResult[] = [];
				const replacedFilesToDelete = new Set<string>();

				/* eslint-disable no-await-in-loop */
				for (const item of args.mods) {
					try {
						const versionInfo = await queryClient.fetchQuery(
							modQueries.versionInfo(item.modId, item.version)
						);
						const target = resolveDownloadTarget(
							item.modId,
							item.version,
							versionInfo,
							settings.game_platform
						);
						const destination = await join(profile.path, 'BepInEx', 'plugins', target.fileName);
						await rustInvoke('modding_mod_download', {
							modId: item.modId,
							url: target.url,
							destination,
							expectedChecksum: target.checksum
						});

						installed.push({
							mod_id: item.modId,
							version: item.version,
							file_name: target.fileName
						});

						await rustInvoke('profiles_add_mod', {
							profileId: args.profileId,
							modId: item.modId,
							version: item.version,
							file: target.fileName
						});

						persisted.push({
							mod_id: item.modId,
							version: item.version,
							file_name: target.fileName
						});

						const previous = previousByModId.get(item.modId);
						if (previous?.file && previous.file !== target.fileName) {
							replacedFilesToDelete.add(previous.file);
						}
					} catch (error) {
						await rollbackInstalledMods(
							args.profileId,
							profile.path,
							installed,
							persisted,
							previousByModId
						);
						throw error;
					}
				}
				/* eslint-enable no-await-in-loop */
				await Promise.all(
					Array.from(replacedFilesToDelete).map((fileName) =>
						rustInvoke('profiles_delete_mod_file', {
							profilePath: profile.path,
							fileName
						}).catch((error) => {
							console.warn('[installMods] Failed to delete replaced mod file', {
								profilePath: profile.path,
								fileName,
								error
							});
						})
					)
				);

				return installed;
			} catch (error) {
				failed = true;
				const message = error instanceof Error ? error.message : 'Unknown error';
				for (const item of args.mods) {
					if (!installed.some((entry) => entry.mod_id === item.modId)) {
						gameState.setModDownloadError(item.modId, message);
					}
				}
				throw error;
			} finally {
				modsInstallInFlight.delete(args.profileId);
				unlistenModDownload?.();
				if (!failed) {
					for (const item of args.mods) {
						gameState.clearModDownload(item.modId);
					}
				}
			}
		},
		onSuccess: (_data: InstalledModResult[], args: InstallArgs) => {
			void invalidateProfileAndDiskQueries(queryClient, args);
		}
	}),

	launchProfile: (queryClient?: QueryClient) => ({
		mutationFn: async (profile: Profile) => {
			if (launchInFlight) {
				throw new Error('A launch is already in progress');
			}
			launchInFlight = true;
			try {
				const settings = await rustInvoke('core_get_settings');
				if (!settings.among_us_path?.trim()) {
					throw new Error('Among Us path not configured');
				}
				if (!settings.allow_multi_instance_launch && gameState.running) {
					throw new Error('An Among Us instance is already running');
				}
				if (settings.game_platform === 'epic') {
					await epicService.ensureLoggedIn();
				}

				if (settings.game_platform === 'xbox') {
					let appId = settings.xbox_app_id?.trim() ?? '';
					if (!appId) {
						appId = await rustInvoke('game_xbox_get_app_id');
						await rustInvoke('core_update_settings', { updates: { xbox_app_id: appId } });
						queryClient?.setQueryData<AppSettings | undefined>(settingsQueryKey, (current) =>
							current ? { ...current, xbox_app_id: appId } : current
						);
					}
					await rustInvoke('game_xbox_prepare_launch', {
						gameDir: settings.among_us_path,
						profilePath: profile.path
					});
					await rustInvoke('game_xbox_launch', {
						appId,
						profileId: profile.id
					});
				} else {
					const gameExe = await join(settings.among_us_path, 'Among Us.exe');
					await assertPathExists(gameExe, 'Among Us.exe not found at configured path');
					const bepinexDll = await join(
						profile.path,
						'BepInEx',
						'core',
						'BepInEx.Unity.IL2CPP.dll'
					);
					await assertPathExists(
						bepinexDll,
						'BepInEx DLL not found. Please wait for installation to complete.'
					);
					const dotnetDir = await join(profile.path, 'dotnet');
					const coreclrPath = await join(dotnetDir, 'coreclr.dll');
					await assertPathExists(
						coreclrPath,
						'dotnet runtime not found. Please wait for installation to complete.'
					);
					await rustInvoke('game_launch_modded', {
						gameExe,
						profileId: profile.id,
						profilePath: profile.path,
						bepinexDll,
						dotnetDir,
						coreclrPath,
						platform: settings.game_platform
					});
				}
				try {
					await rustInvoke('profiles_update_last_launched', { profileId: profile.id });
				} catch {
					// Best-effort bookkeeping; launch already succeeded.
				}

				if (settings.close_on_launch) {
					const { getCurrentWindow } = await import('@tauri-apps/api/window');
					await getCurrentWindow().close();
				}
			} finally {
				launchInFlight = false;
			}
		}
	}),

	launchVanilla: (queryClient?: QueryClient) => ({
		mutationFn: async () => {
			if (launchInFlight) {
				throw new Error('A launch is already in progress');
			}
			launchInFlight = true;
			try {
				const settings = await rustInvoke('core_get_settings');
				if (!settings.among_us_path?.trim()) {
					throw new Error('Among Us path not configured');
				}
				if (!settings.allow_multi_instance_launch && gameState.running) {
					throw new Error('An Among Us instance is already running');
				}
				if (settings.game_platform === 'epic') {
					await epicService.ensureLoggedIn();
				}

				if (settings.game_platform === 'xbox') {
					let appId = settings.xbox_app_id?.trim() ?? '';
					if (!appId) {
						appId = await rustInvoke('game_xbox_get_app_id');
						await rustInvoke('core_update_settings', { updates: { xbox_app_id: appId } });
						queryClient?.setQueryData<AppSettings | undefined>(settingsQueryKey, (current) =>
							current ? { ...current, xbox_app_id: appId } : current
						);
					}
					await rustInvoke('game_xbox_cleanup', { gameDir: settings.among_us_path });
					await rustInvoke('game_xbox_launch', { appId, profileId: null });
				} else {
					const gameExe = await join(settings.among_us_path, 'Among Us.exe');
					await assertPathExists(gameExe, 'Among Us.exe not found at configured path');
					await rustInvoke('game_launch_vanilla', {
						gameExe,
						platform: settings.game_platform
					});
				}

				if (settings.close_on_launch) {
					const { getCurrentWindow } = await import('@tauri-apps/api/window');
					await getCurrentWindow().close();
				}
			} finally {
				launchInFlight = false;
			}
		}
	})
};

export type CreateProfileMutation = ReturnType<typeof profileMutations.create>;
export type DeleteProfileMutation = ReturnType<typeof profileMutations.delete>;
