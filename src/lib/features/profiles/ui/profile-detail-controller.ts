import { goto } from '$app/navigation';
import { resolve } from '$app/paths';
import { join } from '@tauri-apps/api/path';
import { revealItemInDir } from '@tauri-apps/plugin-opener';
import type { Profile, UnifiedMod } from '$lib/features/profiles/schema';
import { rememberInstallTarget } from '$lib/features/mods/state/install-target.svelte';
import { showError, showSuccess } from '$lib/utils/toast';

interface ProfileDetailControllerDeps {
	launchProfile: (profile: Profile) => Promise<void>;
	updateLastLaunched: (profileId: string) => Promise<void>;
	deleteProfile: (profileId: string) => Promise<void>;
	removeProfileQueries: (profileId: string) => void;
	renameProfile: (profileId: string, newName: string) => Promise<void>;
	deleteUnifiedMod: (profileId: string, mod: UnifiedMod) => Promise<void>;
}

export function createProfileDetailController(deps: ProfileDetailControllerDeps) {
	return {
		launchProfile: async (profile: Profile) => {
			await deps.launchProfile(profile);
			await deps.updateLastLaunched(profile.id);
		},

		openProfileFolder: async (profile: Profile) => {
			try {
				await revealItemInDir(await join(profile.path, 'BepInEx'));
			} catch (error) {
				showError(error, 'Open folder');
			}
		},

		deleteProfile: async (profile: Profile) => {
			await deps.deleteProfile(profile.id);
			deps.removeProfileQueries(profile.id);
			showSuccess(`Profile "${profile.name}" deleted`);
			goto(resolve('/library'));
		},

		renameProfile: (profile: Profile, newName: string) =>
			deps.renameProfile(profile.id, newName).then(() => {
				showSuccess('Profile renamed');
			}),

		removeMod: async (profile: Profile, mod: UnifiedMod) => {
			await deps.deleteUnifiedMod(profile.id, mod);
			showSuccess('Mod removed');
		},

		goToInstallMods: (profileId: string) => {
			rememberInstallTarget(profileId, 'install-click');
			goto(resolve('/explore'));
		}
	};
}
