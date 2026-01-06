import { Store } from '@tauri-apps/plugin-store';
import { appDataDir, join } from '@tauri-apps/api/path';
import { info, debug } from '@tauri-apps/plugin-log';
import type { AppSettings } from './schema';

class SettingsService {
	async getSettings(): Promise<AppSettings> {
		const store = await Store.load('registry.json');
		const settings = await store.get<AppSettings>('settings');

		if (!settings) {
			debug('No settings found, using defaults');
		}

		return (
			settings ?? {
				bepinex_url:
					'https://builds.bepinex.dev/projects/bepinex_be/738/BepInEx-Unity.IL2CPP-win-x86-6.0.0-be.738%2Baf0cba7.zip',
				among_us_path: '',
				close_on_launch: false,
				game_platform: 'steam',
				cache_bepinex: false,
				copy_game_files: 'ignore'
			}
		);
	}

	async updateSettings(updates: Partial<AppSettings>): Promise<void> {
		info(`Updating settings: ${Object.keys(updates).join(', ')}`);
		const store = await Store.load('registry.json');
		const current = await this.getSettings();
		await store.set('settings', { ...current, ...updates });
		await store.save();
		debug('Settings saved');
	}

	async getBepInExCachePath(): Promise<string> {
		const dataDir = await appDataDir();
		const cacheDir = await join(dataDir, 'cache');
		return await join(cacheDir, 'bepinex.zip');
	}
}

export const settingsService = new SettingsService();
