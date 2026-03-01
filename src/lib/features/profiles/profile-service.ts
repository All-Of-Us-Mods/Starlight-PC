import type { BepInExProgress, Profile, ProfileIconSelection, UnifiedMod } from './schema';
import { profileWorkflowService } from './profile-workflow-service';
import { gameState } from './game-state.svelte';
import { notifyProfilesInvalidated } from './state/profile-invalidation-bridge';
import { showError } from '$lib/utils/toast';
import { error as logError } from '@tauri-apps/plugin-log';

class ProfileServiceFacade {
	readonly getProfilesDir = () => profileWorkflowService.getProfilesDir();
	readonly getProfiles = () => profileWorkflowService.getProfiles();
	readonly getProfileById = (id: string) => profileWorkflowService.getProfileById(id);

	private buildLifecycleHooks() {
		return {
			onBepInExProgress: (profileId: string, progress: BepInExProgress) => {
				gameState.setBepInExProgress(profileId, progress);
			},
			onBepInExError: (profileId: string, message: string) => {
				gameState.setBepInExError(profileId, message);
				showError(new Error(message), 'BepInEx installation');
			},
			onBepInExInstalled: () => {
				notifyProfilesInvalidated();
			},
			onBepInExDone: (profileId: string) => {
				gameState.clearBepInExProgress(profileId);
			}
		};
	}

	async createProfile(name: string): Promise<Profile> {
		try {
			return await profileWorkflowService.createProfile(name, this.buildLifecycleHooks());
		} catch (error) {
			logError(`Failed to create profile: ${error instanceof Error ? error.message : error}`);
			throw error;
		}
	}

	async retryBepInExInstall(profileId: string, profilePath: string): Promise<void> {
		gameState.clearBepInExProgress(profileId);
		await profileWorkflowService.retryBepInExInstall(
			profileId,
			profilePath,
			this.buildLifecycleHooks()
		);
	}

	async deleteProfile(profileId: string): Promise<void> {
		gameState.clearBepInExProgress(profileId);
		await profileWorkflowService.deleteProfile(profileId);
	}

	readonly renameProfile = (profileId: string, newName: string) =>
		profileWorkflowService.renameProfile(profileId, newName);
	readonly updateProfileIcon = (profileId: string, selection: ProfileIconSelection) =>
		profileWorkflowService.updateProfileIcon(profileId, selection);
	readonly getActiveProfile = () => profileWorkflowService.getActiveProfile();
	readonly updateLastLaunched = (profileId: string) =>
		profileWorkflowService.updateLastLaunched(profileId);
	readonly addModToProfile = (profileId: string, modId: string, version: string, file: string) =>
		profileWorkflowService.addModToProfile(profileId, modId, version, file);
	readonly addPlayTime = (profileId: string, durationMs: number) =>
		profileWorkflowService.addPlayTime(profileId, durationMs);
	readonly removeModFromProfile = (profileId: string, modId: string) =>
		profileWorkflowService.removeModFromProfile(profileId, modId);
	readonly getModFiles = (profilePath: string) => profileWorkflowService.getModFiles(profilePath);
	readonly countMods = (profilePath: string) => profileWorkflowService.countMods(profilePath);
	readonly deleteModFile = (profilePath: string, fileName: string) =>
		profileWorkflowService.deleteModFile(profilePath, fileName);
	readonly getUnifiedMods = (profileId: string) => profileWorkflowService.getUnifiedMods(profileId);
	readonly cleanupMissingMods = (profileId: string) =>
		profileWorkflowService.cleanupMissingMods(profileId);
	readonly deleteUnifiedMod = (profileId: string, mod: UnifiedMod) =>
		profileWorkflowService.deleteUnifiedMod(profileId, mod);
}

export const profileService = new ProfileServiceFacade();
