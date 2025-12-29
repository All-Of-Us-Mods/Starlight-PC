import { writeFile, mkdir, remove } from '@tauri-apps/plugin-fs';
import { join } from '@tauri-apps/api/path';
import { apiFetch } from '$lib/api/client';
import { ModVersionInfo, ModVersion } from '../mods/schema';
import { type } from 'arktype';

const ModVersionsArray = type(ModVersion.array());

class ModInstallService {
	async getModVersions(modId: string): Promise<ModVersion[]> {
		return await apiFetch(`/api/v2/mods/${modId}/versions`, ModVersionsArray);
	}

	async getModVersionInfo(modId: string, version: string): Promise<ModVersionInfo> {
		return await apiFetch(`/api/v2/mods/${modId}/versions/${version}/info`, type(ModVersionInfo));
	}

	async installModToProfile(modId: string, version: string, profilePath: string): Promise<string> {
		const info = await this.getModVersionInfo(modId, version);
		const response = await fetch(info.download_url);
		if (!response.ok) throw new Error('Download failed');

		const data = new Uint8Array(await response.arrayBuffer());

		const hashBuffer = await crypto.subtle.digest('SHA-256', data);
		const hashArray = Array.from(new Uint8Array(hashBuffer));
		const hashHex = hashArray.map((b) => b.toString(16).padStart(2, '0')).join('');

		if (hashHex !== info.checksum) {
			throw new Error(`Checksum mismatch: expected ${info.checksum}, got ${hashHex}`);
		}

		const pluginsDir = await join(profilePath, 'BepInEx', 'plugins');

		await mkdir(pluginsDir, { recursive: true });
		await writeFile(await join(pluginsDir, info.file_name), data);

		return info.file_name;
	}

	async removeModFromProfile(fileName: string, profilePath: string): Promise<void> {
		const dllPath = await join(profilePath, 'BepInEx', 'plugins', fileName);
		await remove(dllPath);
	}
}

export const modInstallService = new ModInstallService();
