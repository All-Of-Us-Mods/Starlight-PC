import { appDataDir, join } from '@tauri-apps/api/path';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { mkdir, readDir, readTextFile, remove, writeTextFile } from '@tauri-apps/plugin-fs';
import type { BepInExProgress } from './schema';

class ProfilePlatformAdapter {
	getAppDataDir() {
		return appDataDir();
	}

	joinPath(...parts: string[]) {
		if (parts.length === 0) throw new Error('No path parts provided');
		const [first, ...rest] = parts;
		return join(first, ...rest);
	}

	ensureDir(path: string) {
		return mkdir(path, { recursive: true });
	}

	removePath(path: string) {
		return remove(path, { recursive: true });
	}

	readDirectory(path: string) {
		return readDir(path);
	}

	async readJsonFile<T>(path: string): Promise<T> {
		const content = await readTextFile(path);
		return JSON.parse(content) as T;
	}

	readTextFile(path: string) {
		return readTextFile(path);
	}

	writeJsonFile(path: string, data: unknown) {
		return writeTextFile(path, JSON.stringify(data, null, 2));
	}

	listenBepInExProgress(callback: (progress: BepInExProgress) => void): Promise<UnlistenFn> {
		return listen<BepInExProgress>('bepinex-progress', (event) => callback(event.payload));
	}

	installBepInEx(args: { url: string; destination: string; cachePath: string | null }) {
		return invoke('install_bepinex', args);
	}

	exportProfileZip(args: { profilePath: string; destination: string }) {
		return invoke('export_profile_zip', args);
	}

	importProfileZip(args: { zipPath: string; destination: string }) {
		return invoke<{ metadata_name?: string | null }>('import_profile_zip', args);
	}
}

export const profilePlatformAdapter = new ProfilePlatformAdapter();
