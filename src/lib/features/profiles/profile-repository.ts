import { type } from 'arktype';
import { getStore } from '$lib/state/store';
import { debug, info, warn } from '@tauri-apps/plugin-log';
import { ProfileEntry, type Profile } from './schema';
import { profilePlatformAdapter } from './profile-platform-adapter';

const ProfilesArray = type(ProfileEntry.array());

class ProfileRepository {
	private migrationDone = false;

	async getProfilesDir(): Promise<string> {
		const dataDir = await profilePlatformAdapter.getAppDataDir();
		const profilesDir = await profilePlatformAdapter.joinPath(dataDir, 'profiles');
		await profilePlatformAdapter.ensureDir(profilesDir);
		return profilesDir;
	}

	private async getMetadataPath(profileDir: string): Promise<string> {
		return profilePlatformAdapter.joinPath(profileDir, 'metadata.json');
	}

	async readMetadata(profileDir: string): Promise<Profile | null> {
		try {
			const metadataPath = await this.getMetadataPath(profileDir);
			const raw = await profilePlatformAdapter.readJsonFile<Record<string, unknown>>(metadataPath);
			const result = ProfileEntry({ ...raw, path: profileDir });
			if (result instanceof type.errors) {
				warn(`ProfileEntry validation failed for ${profileDir}: ${result.summary}`);
				return null;
			}
			return result;
		} catch (error) {
			const message = error instanceof Error ? error.message : String(error);
			warn(`Failed to read metadata for ${profileDir}: ${message}`);
			return null;
		}
	}

	async writeMetadata(profile: Profile): Promise<void> {
		const metadataPath = await this.getMetadataPath(profile.path);
		await profilePlatformAdapter.writeJsonFile(metadataPath, profile);
	}

	async getProfiles(): Promise<Profile[]> {
		const profilesDir = await this.getProfilesDir();
		let entries: Awaited<ReturnType<typeof profilePlatformAdapter.readDirectory>>;
		try {
			entries = await profilePlatformAdapter.readDirectory(profilesDir);
		} catch {
			debug('Failed to read profiles directory');
			return [];
		}

		let legacyMap: Map<string, Profile> | null = null;
		if (!this.migrationDone) {
			const store = await getStore();
			const rawLegacy = await store.get('profiles');
			const legacyResult = ProfilesArray(rawLegacy ?? []);
			const legacyProfiles = legacyResult instanceof type.errors ? [] : legacyResult;

			if (legacyProfiles.length > 0) {
				legacyMap = new Map(legacyProfiles.map((profile) => [profile.id, profile]));
			} else {
				this.migrationDone = true;
			}
		}

		const profiles: Profile[] = [];
		const pendingMigrations: Profile[] = [];

		const directoryEntries = entries.filter((entry) => entry.isDirectory);

		// Resolve all profile dirs and read metadata in parallel
		const profileResults = await Promise.all(
			directoryEntries.map(async (entry) => {
				const profileDir = await profilePlatformAdapter.joinPath(profilesDir, entry.name);
				const profile = await this.readMetadata(profileDir);
				return { entry, profileDir, profile };
			})
		);

		for (const { entry, profileDir, profile: readProfile } of profileResults) {
			let profile = readProfile;

			if (!profile && legacyMap) {
				const legacy = legacyMap.get(entry.name);
				if (legacy) {
					profile = { ...legacy, path: profileDir };
					pendingMigrations.push(profile);
					info(`Migrated profile ${entry.name} to metadata.json`);
				}
			}

			if (!profile) continue;
			profiles.push(profile);
		}

		// Write all pending migrations in parallel
		if (pendingMigrations.length > 0) {
			await Promise.all(pendingMigrations.map((profile) => this.writeMetadata(profile)));
		}
		const migratedCount = pendingMigrations.length;

		if (legacyMap && migratedCount > 0) {
			const allMigrated = [...legacyMap.keys()].every((id) =>
				profiles.some((profile) => profile.id === id)
			);
			if (allMigrated) {
				const store = await getStore();
				await store.set('profiles', []);
				await store.save();
				this.migrationDone = true;
				info('All profiles migrated to metadata.json — cleared legacy registry entry');
			}
		} else if (legacyMap) {
			this.migrationDone = true;
		}

		return profiles.toSorted((a, b) => {
			const aLaunched = a.last_launched_at ?? 0;
			const bLaunched = b.last_launched_at ?? 0;
			if (aLaunched !== bLaunched) return bLaunched - aLaunched;
			return b.created_at - a.created_at;
		});
	}

	async getProfileById(id: string): Promise<Profile | undefined> {
		const profilesDir = await this.getProfilesDir();
		const profileDir = await profilePlatformAdapter.joinPath(profilesDir, id);
		return (await this.readMetadata(profileDir)) ?? undefined;
	}

	async createProfileDir(id: string): Promise<string> {
		const profilesDir = await this.getProfilesDir();
		const profilePath = await profilePlatformAdapter.joinPath(profilesDir, id);
		await profilePlatformAdapter.ensureDir(profilePath);
		return profilePath;
	}

	deleteProfileDir(path: string): Promise<void> {
		return profilePlatformAdapter.removePath(path);
	}

	async getModFiles(profilePath: string): Promise<string[]> {
		try {
			const pluginsPath = await profilePlatformAdapter.joinPath(profilePath, 'BepInEx', 'plugins');
			const entries = await profilePlatformAdapter.readDirectory(pluginsPath);
			return entries
				.filter((entry) => !entry.isDirectory && entry.name.toLowerCase().endsWith('.dll'))
				.map((entry) => entry.name);
		} catch {
			return [];
		}
	}

	async deleteModFile(profilePath: string, fileName: string): Promise<void> {
		const pluginsPath = await profilePlatformAdapter.joinPath(profilePath, 'BepInEx', 'plugins');
		const filePath = await profilePlatformAdapter.joinPath(pluginsPath, fileName);
		await profilePlatformAdapter.removePath(filePath);
	}

	async getProfileLog(profilePath: string, fileName = 'LogOutput.log'): Promise<string> {
		try {
			const logPath = await profilePlatformAdapter.joinPath(profilePath, 'BepInEx', fileName);
			return await profilePlatformAdapter.readTextFile(logPath);
		} catch {
			return '';
		}
	}
}

export const profileRepository = new ProfileRepository();
