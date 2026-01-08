import { appDataDir, join } from '@tauri-apps/api/path';
import { exists } from '@tauri-apps/plugin-fs';
import { info, debug } from '@tauri-apps/plugin-log';
import { type } from 'arktype';
import { getStore } from '$lib/state/store';
import { Settings, type AppSettings } from './schema';

const DEFAULT_SETTINGS: AppSettings = {
	bepinex_url:
		'https://builds.bepinex.dev/projects/bepinex_be/738/BepInEx-Unity.IL2CPP-win-x86-6.0.0-be.738%2Baf0cba7.zip',
	among_us_path: '',
	close_on_launch: false,
	game_platform: 'steam',
	cache_bepinex: false
};

class SettingsService {
	async getSettings(): Promise<AppSettings> {
		const store = await getStore();
		const raw = await store.get('settings');

		if (!raw) {
			debug('No settings found, using defaults');
			return DEFAULT_SETTINGS;
		}

		const result = Settings(raw);
		if (result instanceof type.errors) {
			debug('Invalid settings data, using defaults');
			return DEFAULT_SETTINGS;
		}

		return result;
	}

	async updateSettings(updates: Partial<AppSettings>): Promise<void> {
		info(`Updating settings: ${Object.keys(updates).join(', ')}`);
		const store = await getStore();
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

	/**
	 * Auto-detects the game architecture (x86/x64) and updates the BepInEx URL accordingly.
	 * Checks for UnityCrashHandler64.exe to determine if the game is 64-bit.
	 */
	async autoDetectBepInExArchitecture(gamePath: string): Promise<void> {
		const crashHandlerPath = `${gamePath}/UnityCrashHandler64.exe`;
		const settings = await this.getSettings();
		const currentUrl = settings.bepinex_url;

		const is64Bit = await exists(crashHandlerPath);
		const updatedUrl = is64Bit
			? currentUrl.replace(/win-x86(?=-)/i, 'win-x64')
			: currentUrl.replace(/win-x64(?=-)/i, 'win-x86');

		if (updatedUrl !== currentUrl) {
			await this.updateSettings({ bepinex_url: updatedUrl });
			debug(`BepInEx architecture updated to ${is64Bit ? 'x64' : 'x86'}`);
		}
	}
}

export const settingsService = new SettingsService();
