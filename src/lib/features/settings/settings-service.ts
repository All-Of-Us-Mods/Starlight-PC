import { Store } from '@tauri-apps/plugin-store';
import type { AppSettings } from './schema';

class SettingsService {
	async getSettings(): Promise<AppSettings> {
		const store = await Store.load('registry.json');
		return (
			(await store.get<AppSettings>('settings')) ?? {
				bepinex_url:
					'https://builds.bepinex.dev/projects/bepinex_be/738/BepInEx-Unity.IL2CPP-win-x86-6.0.0-be.738%2Baf0cba7.zip',
				bepinex_version: '6.0.0-be.738',
				among_us_path: '',
				close_on_launch: false
			}
		);
	}

	async updateSettings(updates: Partial<AppSettings>): Promise<void> {
		const store = await Store.load('registry.json');
		const current = await this.getSettings();
		await store.set('settings', { ...current, ...updates });
		await store.save();
	}
}

export const settingsService = new SettingsService();
