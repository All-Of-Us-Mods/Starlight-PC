import { mkdir, remove } from '@tauri-apps/plugin-fs';
import { join } from '@tauri-apps/api/path';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { apiFetch } from '$lib/api/client';
import {
	ModVersionInfo as ModVersionInfoSchema,
	ModVersion,
	ModResponse,
	type ModVersionInfo,
	type ModDependency
} from '../mods/schema';
import { type } from 'arktype';
import * as semver from 'semver';

const ModVersionsArray = type(ModVersion.array());
const ModVersionInfoValidator = type(ModVersionInfoSchema);

/** Progress event payload from Rust download_mod command */
export interface ModDownloadProgress {
	mod_id: string;
	downloaded: number;
	total: number | null;
	progress: number; // 0-100
	stage: 'connecting' | 'downloading' | 'verifying' | 'writing' | 'complete';
}

export interface DependencyWithMeta extends ModDependency {
	modName: string;
	resolvedVersion: string;
}

class ModInstallService {
	async getModVersions(modId: string): Promise<ModVersion[]> {
		return await apiFetch(`/api/v2/mods/${modId}/versions`, ModVersionsArray);
	}

	async getModVersionInfo(modId: string, version: string): Promise<ModVersionInfo> {
		return await apiFetch(
			`/api/v2/mods/${modId}/versions/${version}/info`,
			ModVersionInfoValidator
		);
	}

	async getModById(modId: string): Promise<typeof ModResponse.infer> {
		return await apiFetch(`/api/v2/mods/${modId}`, ModResponse);
	}

	/**
	 * Resolves dependencies with their mod names and latest matching versions
	 */
	async resolveDependencies(dependencies: ModDependency[]): Promise<DependencyWithMeta[]> {
		const resolved: DependencyWithMeta[] = [];

		for (const dep of dependencies) {
			try {
				const [mod, versions] = await Promise.all([
					this.getModById(dep.mod_id),
					this.getModVersions(dep.mod_id)
				]);

				// Find the latest version that satisfies the constraint
				const sortedVersions = [...versions].sort((a, b) => b.created_at - a.created_at);
				let resolvedVersion = sortedVersions[0]?.version ?? '';

				// Try to find a version matching the constraint using semver
				if (dep.version_constraint && dep.version_constraint !== '*') {
					for (const v of sortedVersions) {
						try {
							if (semver.satisfies(v.version, dep.version_constraint)) {
								resolvedVersion = v.version;
								break;
							}
						} catch {
							// If semver parsing fails, fall back to latest
						}
					}
				}

				resolved.push({
					...dep,
					modName: mod.name,
					resolvedVersion
				});
			} catch {
				// If we can't resolve a dependency, skip it
				console.warn(`Failed to resolve dependency: ${dep.mod_id}`);
			}
		}

		return resolved;
	}

	/**
	 * Listens for mod download progress events
	 * Returns an unlisten function to stop listening
	 */
	async onDownloadProgress(callback: (progress: ModDownloadProgress) => void): Promise<UnlistenFn> {
		return await listen<ModDownloadProgress>('mod-download-progress', (event) => {
			callback(event.payload);
		});
	}

	async installModToProfile(modId: string, version: string, profilePath: string): Promise<string> {
		const info = await this.getModVersionInfo(modId, version);

		const pluginsDir = await join(profilePath, 'BepInEx', 'plugins');
		await mkdir(pluginsDir, { recursive: true });

		const destination = await join(pluginsDir, info.file_name);

		await invoke('download_mod', {
			modId,
			url: info.download_url,
			destination,
			expectedChecksum: info.checksum
		});

		return info.file_name;
	}

	/**
	 * Installs multiple mods to a profile
	 * Returns an array of { modId, version, fileName } for successful installs
	 */
	async installModsToProfile(
		mods: Array<{ modId: string; version: string }>,
		profilePath: string
	): Promise<Array<{ modId: string; version: string; fileName: string }>> {
		const results: Array<{ modId: string; version: string; fileName: string }> = [];

		for (const mod of mods) {
			const fileName = await this.installModToProfile(mod.modId, mod.version, profilePath);
			results.push({ modId: mod.modId, version: mod.version, fileName });
		}

		return results;
	}

	async removeModFromProfile(fileName: string, profilePath: string): Promise<void> {
		const dllPath = await join(profilePath, 'BepInEx', 'plugins', fileName);
		await remove(dllPath);
	}
}

export const modInstallService = new ModInstallService();
