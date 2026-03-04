import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { error as logError } from '@tauri-apps/plugin-log';
import type { BepInExProgress, Profile, ProfileIconSelection, UnifiedMod } from './schema';

interface ProfileLifecycleHooks {
	onBepInExProgress?: (profileId: string, progress: BepInExProgress) => void;
	onBepInExError?: (profileId: string, message: string) => void;
	onBepInExInstalled?: (profileId: string) => void;
	onBepInExDone?: (profileId: string) => void;
}

class ProfileWorkflowService {
	readonly getProfilesDir = () => invoke<string>('profiles_get_dir');
	readonly getProfiles = () => invoke<Profile[]>('profiles_list');
	readonly getProfileById = (id: string) =>
		invoke<Profile | null>('profiles_get_by_id', { args: { id } }).then((profile) => profile ?? undefined);

	async createProfile(name: string, hooks?: ProfileLifecycleHooks): Promise<Profile> {
		const profile = await invoke<Profile>('profiles_create', { args: { name } });

		this.installBepInExInBackground(profile.id, profile.path, hooks).catch((error) => {
			logError(
				`installBepInExInBackground failed: ${error instanceof Error ? error.message : error}`
			);
		});
		return profile;
	}

	async exportProfileZip(profileId: string, destination: string): Promise<void> {
		await invoke<void>('profiles_export_zip', { args: { profileId, destination } });
	}

	async importProfileZip(zipPath: string): Promise<Profile> {
		return await invoke<Profile>('profiles_import_zip', { args: { zipPath } });
	}

	async retryBepInExInstall(profileId: string, profilePath: string, hooks?: ProfileLifecycleHooks) {
		hooks?.onBepInExDone?.(profileId);
		await this.installBepInExInBackground(profileId, profilePath, hooks);
	}

	private async installBepInExInBackground(
		profileId: string,
		profilePath: string,
		hooks?: ProfileLifecycleHooks
	): Promise<void> {
		let unlisten: (() => void) | undefined;
		try {
			if (hooks?.onBepInExProgress) {
				unlisten = await listen<BepInExProgress>('bepinex-progress', (event) => {
					hooks.onBepInExProgress?.(profileId, event.payload);
				});
			}

			await invoke<void>('profiles_install_bepinex', { args: { profileId, profilePath } });
			hooks?.onBepInExInstalled?.(profileId);
		} catch (error) {
			const message = error instanceof Error ? error.message : 'Unknown error';
			hooks?.onBepInExError?.(profileId, message);
			throw error;
		} finally {
			unlisten?.();
			hooks?.onBepInExDone?.(profileId);
		}
	}

	readonly deleteProfile = (profileId: string) =>
		invoke<void>('profiles_delete', { args: { profileId } });
	readonly renameProfile = (profileId: string, newName: string) =>
		invoke<void>('profiles_rename', { args: { profileId, newName } });
	readonly updateProfileIcon = (profileId: string, selection: ProfileIconSelection) =>
		invoke<void>('profiles_update_icon', { args: { profileId, selection } });
	readonly getActiveProfile = () => invoke<Profile | null>('profiles_get_active');
	readonly updateLastLaunched = (profileId: string) =>
		invoke<void>('profiles_update_last_launched', { args: { profileId } });
	readonly addModToProfile = (profileId: string, modId: string, version: string, file: string) =>
		invoke<void>('profiles_add_mod', { args: { profileId, modId, version, file } });
	readonly addPlayTime = (profileId: string, durationMs: number) =>
		invoke<void>('profiles_add_play_time', { args: { profileId, durationMs } });
	readonly removeModFromProfile = (profileId: string, modId: string) =>
		invoke<void>('profiles_remove_mod', { args: { profileId, modId } });
	readonly getModFiles = (profilePath: string) =>
		invoke<string[]>('profiles_get_mod_files', { args: { profilePath } });
	readonly countMods = async (profilePath: string) => (await this.getModFiles(profilePath)).length;
	readonly deleteModFile = (profilePath: string, fileName: string) =>
		invoke<void>('profiles_delete_mod_file', { args: { profilePath, fileName } });
	readonly getProfileLog = (profilePath: string, fileName = 'LogOutput.log') =>
		invoke<string>('profiles_get_log', { args: { profilePath, fileName } });
	readonly getUnifiedMods = (profileId: string) =>
		invoke<UnifiedMod[]>('profiles_get_unified_mods', { args: { profileId } });
	readonly cleanupMissingMods = (profileId: string) =>
		invoke<void>('profiles_cleanup_missing_mods', { args: { profileId } });
	readonly deleteUnifiedMod = (profileId: string, mod: UnifiedMod) =>
		invoke<void>('profiles_delete_unified_mod', { args: { profileId, modEntry: mod } });
}

export const profileWorkflowService = new ProfileWorkflowService();
