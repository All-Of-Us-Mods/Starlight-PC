import { error as logError } from '@tauri-apps/plugin-log';
import { settingsService } from '../settings/settings-service';
import { profileRepository } from './profile-repository';
import { profilePlatformAdapter } from './profile-platform-adapter';
import type { BepInExProgress, Profile, UnifiedMod } from './schema';

interface ProfileLifecycleHooks {
	onBepInExProgress?: (profileId: string, progress: BepInExProgress) => void;
	onBepInExError?: (profileId: string, message: string) => void;
	onBepInExInstalled?: (profileId: string) => void;
	onBepInExDone?: (profileId: string) => void;
}

class ProfileWorkflowService {
	readonly getProfilesDir = () => profileRepository.getProfilesDir();
	readonly getProfiles = () => profileRepository.getProfiles();
	readonly getProfileById = (id: string) => profileRepository.getProfileById(id);

	async createProfile(name: string, hooks?: ProfileLifecycleHooks): Promise<Profile> {
		const trimmed = name.trim();
		if (!trimmed) throw new Error('Profile name cannot be empty');

		const profiles = await this.getProfiles();
		if (profiles.some((profile) => profile.name.toLowerCase() === trimmed.toLowerCase())) {
			throw new Error(`Profile '${trimmed}' already exists`);
		}

		const timestamp = Date.now();
		const profileId = this.buildProfileId(trimmed, timestamp);
		const profilePath = await profileRepository.createProfileDir(profileId);

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

		await profileRepository.writeMetadata(profile);
		this.installBepInExInBackground(profileId, profilePath, hooks).catch((error) => {
			logError(
				`installBepInExInBackground failed: ${error instanceof Error ? error.message : error}`
			);
		});
		return profile;
	}

	async exportProfileZip(profileId: string, destination: string): Promise<void> {
		const profile = await this.getProfileById(profileId);
		if (!profile) throw new Error(`Profile '${profileId}' not found`);
		await profilePlatformAdapter.exportProfileZip({
			profilePath: profile.path,
			destination
		});
	}

	async importProfileZip(zipPath: string): Promise<Profile> {
		const profiles = await this.getProfiles();
		const zipName = this.deriveNameFromZipPath(zipPath);
		const timestamp = Date.now();
		const profileId = this.buildProfileId(zipName, timestamp);
		const profilePath = await profileRepository.createProfileDir(profileId);

		try {
			const result = await profilePlatformAdapter.importProfileZip({
				zipPath,
				destination: profilePath
			});

			const importedMetadata = await this.readImportedMetadata(profilePath);
			const requestedName = result?.metadata_name ?? importedMetadata.name ?? zipName;
			const uniqueName = this.makeUniqueProfileName(requestedName, profiles);

			const profile: Profile = {
				id: profileId,
				name: uniqueName,
				path: profilePath,
				created_at: timestamp,
				last_launched_at: importedMetadata.lastLaunchedAt,
				bepinex_installed: true,
				total_play_time: 0,
				mods: importedMetadata.mods
			};

			await profileRepository.writeMetadata(profile);
			return profile;
		} catch (error) {
			await profileRepository.deleteProfileDir(profilePath);
			throw error;
		}
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
		const bepinexUrl = await this.getBepInExUrl();

		try {
			await this.downloadBepInEx(profilePath, bepinexUrl, (progress) => {
				hooks?.onBepInExProgress?.(profileId, progress);
			});

			const profile = await this.getProfileById(profileId);
			if (profile) {
				profile.bepinex_installed = true;
				await profileRepository.writeMetadata(profile);
				hooks?.onBepInExInstalled?.(profileId);
			}
		} catch (error) {
			const message = error instanceof Error ? error.message : 'Unknown error';
			hooks?.onBepInExError?.(profileId, message);
			throw error;
		} finally {
			hooks?.onBepInExDone?.(profileId);
		}
	}

	private async downloadBepInEx(
		profilePath: string,
		bepinexUrl: string,
		onProgress?: (progress: BepInExProgress) => void
	): Promise<void> {
		let unlisten: (() => void) | undefined;
		try {
			if (onProgress) {
				unlisten = await profilePlatformAdapter.listenBepInExProgress(onProgress);
			}

			const settings = await settingsService.getSettings();
			const cachePath = settings.cache_bepinex ? await settingsService.getBepInExCachePath() : null;
			await profilePlatformAdapter.installBepInEx({
				url: bepinexUrl,
				destination: profilePath,
				cachePath
			});
		} finally {
			unlisten?.();
		}
	}

	async deleteProfile(profileId: string): Promise<void> {
		const profile = await this.getProfileById(profileId);
		if (!profile) throw new Error(`Profile '${profileId}' not found`);
		await profileRepository.deleteProfileDir(profile.path);
	}

	async renameProfile(profileId: string, newName: string): Promise<void> {
		const trimmed = newName.trim();
		if (!trimmed) throw new Error('Profile name cannot be empty');

		const profiles = await this.getProfiles();
		const profile = profiles.find((candidate) => candidate.id === profileId);
		if (!profile) throw new Error(`Profile '${profileId}' not found`);

		if (
			profiles.some(
				(candidate) =>
					candidate.id !== profileId && candidate.name.toLowerCase() === trimmed.toLowerCase()
			)
		) {
			throw new Error(`Profile '${trimmed}' already exists`);
		}

		profile.name = trimmed;
		await profileRepository.writeMetadata(profile);
	}

	async getActiveProfile(): Promise<Profile | null> {
		const profiles = await this.getProfiles();
		const profilesWithLaunched = profiles.filter(
			(profile) => profile.last_launched_at !== undefined
		);
		if (profilesWithLaunched.length === 0) return null;

		return profilesWithLaunched.reduce((latest, profile) => {
			const latestTime = latest.last_launched_at ?? 0;
			const profileTime = profile.last_launched_at ?? 0;
			return profileTime > latestTime ? profile : latest;
		});
	}

	async updateLastLaunched(profileId: string): Promise<void> {
		const profile = await this.getProfileById(profileId);
		if (!profile) return;
		profile.last_launched_at = Date.now();
		await profileRepository.writeMetadata(profile);
	}

	async addModToProfile(
		profileId: string,
		modId: string,
		version: string,
		file: string
	): Promise<void> {
		const profile = await this.getProfileById(profileId);
		if (!profile) throw new Error(`Profile '${profileId}' not found`);

		const modIndex = profile.mods.findIndex((mod) => mod.mod_id === modId);
		if (modIndex >= 0) {
			profile.mods[modIndex] = { mod_id: modId, version, file };
		} else {
			profile.mods.push({ mod_id: modId, version, file });
		}
		await profileRepository.writeMetadata(profile);
	}

	async addPlayTime(profileId: string, durationMs: number): Promise<void> {
		const profile = await this.getProfileById(profileId);
		if (!profile) throw new Error(`Profile '${profileId}' not found`);
		profile.total_play_time = (profile.total_play_time ?? 0) + durationMs;
		await profileRepository.writeMetadata(profile);
	}

	async removeModFromProfile(profileId: string, modId: string): Promise<void> {
		const profile = await this.getProfileById(profileId);
		if (!profile) throw new Error(`Profile '${profileId}' not found`);
		profile.mods = profile.mods.filter((mod) => mod.mod_id !== modId);
		await profileRepository.writeMetadata(profile);
	}

	readonly getModFiles = (profilePath: string) => profileRepository.getModFiles(profilePath);

	async countMods(profilePath: string): Promise<number> {
		const files = await profileRepository.getModFiles(profilePath);
		return files.length;
	}

	readonly deleteModFile = (profilePath: string, fileName: string) =>
		profileRepository.deleteModFile(profilePath, fileName);
	readonly getProfileLog = (profilePath: string, fileName = 'LogOutput.log') =>
		profileRepository.getProfileLog(profilePath, fileName);

	async getUnifiedMods(profileId: string): Promise<UnifiedMod[]> {
		const profile = await this.getProfileById(profileId);
		if (!profile) throw new Error(`Profile '${profileId}' not found`);

		const diskFiles = await this.getModFiles(profile.path);
		const diskFilesSet = new Set(diskFiles);
		const managedFiles = new Set<string>();

		const unified: UnifiedMod[] = profile.mods
			.filter((mod) => mod.file)
			.filter((mod) => {
				managedFiles.add(mod.file!);
				return diskFilesSet.has(mod.file!);
			})
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

		await this.cleanupMissingManagedMods(profileId, profile, diskFilesSet);
		return unified;
	}

	private async cleanupMissingManagedMods(
		profileId: string,
		profile: Profile,
		diskFiles: Set<string>
	) {
		const missingMods = profile.mods.filter((mod) => mod.file && !diskFiles.has(mod.file));
		if (missingMods.length === 0) return;
		profile.mods = profile.mods.filter((mod) => mod.file && diskFiles.has(mod.file));
		await profileRepository.writeMetadata(profile);
	}

	async cleanupMissingMods(profileId: string): Promise<void> {
		const profile = await this.getProfileById(profileId);
		if (!profile) throw new Error(`Profile '${profileId}' not found`);
		const diskFilesSet = new Set(await this.getModFiles(profile.path));
		const missingMods = profile.mods.filter((mod) => mod.file && !diskFilesSet.has(mod.file));
		if (missingMods.length === 0) return;
		profile.mods = profile.mods.filter((mod) => mod.file && diskFilesSet.has(mod.file));
		await profileRepository.writeMetadata(profile);
	}

	async deleteUnifiedMod(profileId: string, mod: UnifiedMod): Promise<void> {
		const profile = await this.getProfileById(profileId);
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

	private buildProfileId(name: string, timestamp = Date.now()): string {
		const slug = this.slugify(name);
		return slug ? `${slug}-${timestamp}` : `profile-${timestamp}`;
	}

	private deriveNameFromZipPath(zipPath: string): string {
		const parts = zipPath.split(/[\\/]/);
		const fileName = (parts[parts.length - 1] ?? '').trim();
		const withoutZip = fileName.replace(/\.zip$/i, '').trim();
		return withoutZip || `Imported Profile ${new Date().toISOString().slice(0, 10)}`;
	}

	private makeUniqueProfileName(requested: string, profiles: Profile[]): string {
		const base = requested.trim() || 'Imported Profile';
		const names = new Set(profiles.map((profile) => profile.name.toLowerCase()));
		if (!names.has(base.toLowerCase())) return base;

		let suffix = 2;
		let candidate = `${base} (${suffix})`;
		while (names.has(candidate.toLowerCase())) {
			suffix += 1;
			candidate = `${base} (${suffix})`;
		}
		return candidate;
	}

	private async readImportedMetadata(profilePath: string): Promise<{
		name?: string;
		lastLaunchedAt?: number;
		mods: Profile['mods'];
	}> {
		try {
			const metadataPath = await profilePlatformAdapter.joinPath(profilePath, 'metadata.json');
			const raw = await profilePlatformAdapter.readJsonFile<Record<string, unknown>>(metadataPath);

			const name = typeof raw.name === 'string' && raw.name.trim() ? raw.name.trim() : undefined;
			const lastLaunchedAt =
				typeof raw.last_launched_at === 'number' && Number.isFinite(raw.last_launched_at)
					? raw.last_launched_at
					: undefined;
			const mods = this.parseImportedMods(raw.mods);

			return { name, lastLaunchedAt, mods };
		} catch {
			return { mods: [] };
		}
	}

	private parseImportedMods(rawMods: unknown): Profile['mods'] {
		if (!Array.isArray(rawMods)) return [];

		return rawMods
			.map((entry) => {
				if (!entry || typeof entry !== 'object') return null;
				const mod = entry as Record<string, unknown>;
				if (typeof mod.mod_id !== 'string' || typeof mod.version !== 'string') {
					return null;
				}
				const file = typeof mod.file === 'string' ? mod.file : undefined;
				return {
					mod_id: mod.mod_id,
					version: mod.version,
					file
				};
			})
			.filter((entry): entry is NonNullable<typeof entry> => entry !== null);
	}

	private async getBepInExUrl(): Promise<string> {
		const settings = await settingsService.getSettings();
		return settings.bepinex_url;
	}
}

export const profileWorkflowService = new ProfileWorkflowService();
