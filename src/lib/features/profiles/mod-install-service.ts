import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { PUBLIC_API_URL } from '$env/static/public';
import type { ModDependency } from '../mods/schema';
import type { ModDownloadProgress } from './schema';

export interface DependencyWithMeta extends ModDependency {
	modName: string;
	resolvedVersion: string;
}

class ModInstallService {
	async resolveDependencies(dependencies: ModDependency[]): Promise<DependencyWithMeta[]> {
		return invoke<DependencyWithMeta[]>('modding_resolve_dependencies', {
			args: { apiBaseUrl: PUBLIC_API_URL, dependencies }
		});
	}

	async onDownloadProgress(callback: (progress: ModDownloadProgress) => void): Promise<UnlistenFn> {
		return await listen<ModDownloadProgress>('mod-download-progress', (event) => {
			callback(event.payload);
		});
	}
}

export const modInstallService = new ModInstallService();
