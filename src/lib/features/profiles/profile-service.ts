import { Store } from '@tauri-apps/plugin-store';
import { mkdir, remove, readDir } from '@tauri-apps/plugin-fs';
import { join } from '@tauri-apps/api/path';
import { queryClient } from '$lib/state/queryClient';
import type { Profile, UnifiedMod } from './schema';
import { downloadBepInEx } from './bepinex-download';
import { settingsService } from '../settings/settings-service';

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

		const profile: Profile = {
			id: profileId,
			name: trimmed,
			path: profilePath,
			created_at: timestamp,
			last_launched_at: undefined,
			bepinex_installed: false,
			total_play_time: 0,
			mods: []
		};

		profiles.push(profile);
		await store.set('profiles', profiles);
		await store.save();

		this.installBepInExInBackground(profileId, profilePath).catch((err) => {
			console.error(`Failed to install BepInEx for profile ${profileId}:`, err);
		});

		return profile;
	}

	private async installBepInExInBackground(profileId: string, profilePath: string): Promise<void> {
		const bepinexUrl = await this.getBepInExUrl();
		await downloadBepInEx(profilePath, bepinexUrl);

		const store = await this.getStore();
		const profiles = (await store.get<Profile[]>('profiles')) ?? [];
		const profileIndex = profiles.findIndex((p) => p.id === profileId);

		if (profileIndex >= 0) {
			profiles[profileIndex].bepinex_installed = true;
			await store.set('profiles', profiles);
			await store.save();
		}
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

	async addModToProfile(
		profileId: string,
		modId: string,
		version: string,
		file: string
	): Promise<void> {
		const store = await this.getStore();
		const profiles = await this.getProfiles();
		const profile = profiles.find((p) => p.id === profileId);

		if (!profile) throw new Error(`Profile '${profileId}' not found`);

		const modIndex = profile.mods.findIndex((m) => m.mod_id === modId);
		if (modIndex >= 0) {
			profile.mods[modIndex] = { mod_id: modId, version, file };
		} else {
			profile.mods.push({ mod_id: modId, version, file });
		}

		await store.set('profiles', profiles);
		await store.save();
	}

	async addPlayTime(profileId: string, durationMs: number): Promise<void> {
		const store = await this.getStore();
		const profiles = await this.getProfiles();
		const profile = profiles.find((p) => p.id === profileId);

		if (!profile) throw new Error(`Profile '${profileId}' not found`);

		profile.total_play_time = (profile.total_play_time ?? 0) + durationMs;

		await store.set('profiles', profiles);
		await store.save();
		queryClient.invalidateQueries({ queryKey: ['profiles'] });
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

	async getModFiles(profilePath: string): Promise<string[]> {
		try {
			const pluginsPath = await join(profilePath, 'BepInEx', 'plugins');
			const entries = await readDir(pluginsPath);
			return entries.map((entry) => entry.name);
		} catch {
			return [];
		}
	}

	async countMods(profilePath: string): Promise<number> {
		try {
			const pluginsPath = await join(profilePath, 'BepInEx', 'plugins');
			const entries = await readDir(pluginsPath);
			return entries.length;
		} catch {
			return 0;
		}
	}

	async deleteModFile(profilePath: string, fileName: string): Promise<void> {
		const pluginsPath = await join(profilePath, 'BepInEx', 'plugins');
		const filePath = await join(pluginsPath, fileName);
		await remove(filePath, { recursive: true });
	}

	async getUnifiedMods(profileId: string): Promise<UnifiedMod[]> {
		const profiles = await this.getProfiles();
		const profile = profiles.find((p) => p.id === profileId);

		if (!profile) throw new Error(`Profile '${profileId}' not found`);

		const diskFiles = await this.getModFiles(profile.path);
		const managedFiles = new Set(profile.mods.map((m) => m.file).filter(Boolean));

		const unified: UnifiedMod[] = profile.mods
			.filter((m) => m.file)
			.map((mod) => ({
				source: 'managed' as const,
				mod_id: mod.mod_id,
				version: mod.version,
				file: mod.file!
			}));

		for (const file of diskFiles) {
			if (!managedFiles.has(file)) {
				unified.push({ source: 'custom' as const, file });
			}
		}

		return unified;
	}

	async deleteUnifiedMod(profileId: string, mod: UnifiedMod): Promise<void> {
		const profiles = await this.getProfiles();
		const profile = profiles.find((p) => p.id === profileId);

		if (!profile) throw new Error(`Profile '${profileId}' not found`);

		await this.deleteModFile(profile.path, mod.file);

		if (mod.source === 'managed') {
			await this.removeModFromProfile(profileId, mod.mod_id);
		}
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
		const settings = await settingsService.getSettings();
		return settings.bepinex_url;
	}
}

export const profileService = new ProfileService();
