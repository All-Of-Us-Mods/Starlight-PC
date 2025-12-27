import { Store } from '@tauri-apps/plugin-store';
import { mkdir, remove } from '@tauri-apps/plugin-fs';
import { join } from '@tauri-apps/api/path';
import type { Profile } from './schema';
import { downloadBepInEx } from './bepinex-download';

class ProfileService {
	async getStore(): Promise<Store> {
		return await Store.load('registry.json');
	}

	async getProfiles(): Promise<Profile[]> {
		const store = await this.getStore();
		const profiles = (await store.get<Profile[]>('profiles')) ?? [];

		return profiles.sort((a, b) => {
			const aLaunched = a.last_launched_at ?? 0;
			const bLaunched = b.last_launched_at ?? 0;
			if (aLaunched !== bLaunched) return bLaunched - aLaunched;
			return b.created_at - a.created_at;
		});
	}

	async createProfile(name: string): Promise<Profile> {
		const trimmed = name.trim();
		if (!trimmed) throw new Error('Profile name cannot be empty');

		const store = await this.getStore();
		const profiles = await this.getProfiles();

		if (profiles.some((p) => p.name.toLowerCase() === trimmed.toLowerCase())) {
			throw new Error(`Profile '${trimmed}' already exists`);
		}

		const dataDir = await this.getAppDataDir();
		const profilesDir = await join(dataDir, 'profiles');
		const timestamp = Date.now();
		const slug = this.slugify(trimmed);
		const profileId = slug ? `${slug}-${timestamp}` : `profile-${timestamp}`;
		const profilePath = await join(profilesDir, profileId);

		await mkdir(profilePath, { recursive: true });

		const bepinexUrl = await this.getBepInExUrl();
		await downloadBepInEx(profilePath, bepinexUrl);

		const profile: Profile = {
			id: profileId,
			name: trimmed,
			path: profilePath,
			created_at: timestamp,
			last_launched_at: undefined,
			mods: []
		};

		profiles.push(profile);
		await store.set('profiles', profiles);
		await store.save();
		return profile;
	}

	async deleteProfile(profileId: string): Promise<void> {
		const store = await this.getStore();
		const profiles = await this.getProfiles();
		const profile = profiles.find((p) => p.id === profileId);

		if (!profile) throw new Error(`Profile '${profileId}' not found`);

		await remove(profile.path, { recursive: true });
		await store.set(
			'profiles',
			profiles.filter((p) => p.id !== profileId)
		);
		await store.save();
	}

	async getActiveProfile(): Promise<Profile | null> {
		const profiles = await this.getProfiles();
		const profilesWithLaunched = profiles.filter((p) => p.last_launched_at !== undefined);

		if (profilesWithLaunched.length === 0) return null;

		return profilesWithLaunched.reduce((latest, profile) => {
			const latestTime = latest.last_launched_at ?? 0;
			const profileTime = profile.last_launched_at ?? 0;
			return profileTime > latestTime ? profile : latest;
		});
	}

	async updateLastLaunched(profileId: string): Promise<void> {
		const store = await this.getStore();
		const profiles = await this.getProfiles();
		const profile = profiles.find((p) => p.id === profileId);

		if (profile) {
			profile.last_launched_at = Date.now();
			await store.set('profiles', profiles);
			await store.save();
		}
	}

	async addModToProfile(profileId: string, modId: string, version: string): Promise<void> {
		const store = await this.getStore();
		const profiles = await this.getProfiles();
		const profile = profiles.find((p) => p.id === profileId);

		if (!profile) throw new Error(`Profile '${profileId}' not found`);

		const modIndex = profile.mods.findIndex((m) => m.mod_id === modId);
		if (modIndex >= 0) {
			profile.mods[modIndex] = { mod_id: modId, version };
		} else {
			profile.mods.push({ mod_id: modId, version });
		}

		await store.set('profiles', profiles);
		await store.save();
	}

	async removeModFromProfile(profileId: string, modId: string): Promise<void> {
		const store = await this.getStore();
		const profiles = await this.getProfiles();
		const profile = profiles.find((p) => p.id === profileId);

		if (!profile) throw new Error(`Profile '${profileId}' not found`);

		profile.mods = profile.mods.filter((m) => m.mod_id !== modId);
		await store.set('profiles', profiles);
		await store.save();
	}

	private slugify(input: string): string {
		return input
			.toLowerCase()
			.replace(/[^a-z0-9]/g, '-')
			.replace(/-+/g, '-')
			.replace(/^-|-$/g, '');
	}

	private async getAppDataDir(): Promise<string> {
		const { appDataDir } = await import('@tauri-apps/api/path');
		return await appDataDir();
	}

	private async getBepInExUrl(): Promise<string> {
		const store = await this.getStore();
		const settings = await store.get<{ bepinex_url: string }>('settings');
		return settings?.bepinex_url ?? DEFAULT_BEPINEX_URL;
	}
}

const DEFAULT_BEPINEX_URL =
	'https://builds.bepinex.dev/projects/bepinex_be/738/BepInEx-Unity.IL2CPP-win-x86-6.0.0-be.738%2Baf0cba7.zip';
export const profileService = new ProfileService();
