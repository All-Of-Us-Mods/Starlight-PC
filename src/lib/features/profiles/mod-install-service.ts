import { mkdir, remove } from '@tauri-apps/plugin-fs';
import { join } from '@tauri-apps/api/path';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { PUBLIC_API_URL } from '$env/static/public';
import { apiFetch } from '$lib/api/client';
import { warn } from '@tauri-apps/plugin-log';
import {
	ModVersionInfo as ModVersionInfoSchema,
	ModVersion,
	ModResponse,
	type ModVersionInfo,
	type ModDependency
} from '../mods/schema';
import { type ModDownloadProgress } from './schema';
import { type } from 'arktype';
import * as semver from 'semver';
import { settingsService } from '../settings/settings-service';

const ModVersionsArray = type(ModVersion.array());
const ModVersionInfoValidator = type(ModVersionInfoSchema);

export interface DependencyWithMeta extends ModDependency {
	modName: string;
	resolvedVersion: string;
}

class ModInstallService {
	private resolveAbsoluteUrl(pathOrUrl: string): string {
		if (/^https?:\/\//.test(pathOrUrl)) return pathOrUrl;
		return `${PUBLIC_API_URL}${pathOrUrl.startsWith('/') ? '' : '/'}${pathOrUrl}`;
	}

	private async resolveDownloadTarget(
		modId: string,
		version: string,
		info: ModVersionInfo
	): Promise<{ url: string; fileName: string; checksum: string }> {
		const legacyPath = `/api/v2/mods/${modId}/versions/${version}/file`;
		const defaultUrl = info.download_url ?? legacyPath;
		const defaultTarget = {
			url: this.resolveAbsoluteUrl(defaultUrl),
			fileName: info.file_name,
			checksum: info.checksum
		};

		const platforms = info.platforms ?? [];
		if (platforms.length === 0) {
			return defaultTarget;
		}

		const settings = await settingsService.getSettings();
		const architectureFallbacks = settings.game_platform === 'epic' ? ['x64', 'x86'] : ['x86'];

		for (const architecture of architectureFallbacks) {
			const entry = platforms.find(
				(candidate) => candidate.platform === 'windows' && candidate.architecture === architecture
			);
			if (!entry) continue;
			return {
				url: this.resolveAbsoluteUrl(
					entry.download_url ?? `${legacyPath}?platform=windows&arch=${architecture}`
				),
				fileName: entry.file_name ?? info.file_name,
				checksum: entry.checksum ?? info.checksum
			};
		}

		return defaultTarget;
	}

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
				warn(`Failed to resolve dependency: ${dep.mod_id}`);
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
		const target = await this.resolveDownloadTarget(modId, version, info);

		const pluginsDir = await join(profilePath, 'BepInEx', 'plugins');
		await mkdir(pluginsDir, { recursive: true });

		const destination = await join(pluginsDir, target.fileName);

		await invoke('modding_mod_download', {
			args: {
				modId,
				url: target.url,
				destination,
				expectedChecksum: target.checksum
			}
		});

		return target.fileName;
	}

	async removeModFromProfile(fileName: string, profilePath: string): Promise<void> {
		const dllPath = await join(profilePath, 'BepInEx', 'plugins', fileName);
		await remove(dllPath);
	}
}

export const modInstallService = new ModInstallService();
