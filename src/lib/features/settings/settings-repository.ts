import { invoke } from '@tauri-apps/api/core';
import { type } from 'arktype';
import { Settings, type AppSettings } from './schema';

export const DEFAULT_SETTINGS: AppSettings = {
	bepinex_url:
		'https://builds.bepinex.dev/projects/bepinex_be/752/BepInEx-Unity.IL2CPP-win-x86-6.0.0-be.752%2Bdd0655f.zip',
	among_us_path: '',
	close_on_launch: false,
	allow_multi_instance_launch: false,
	game_platform: 'steam',
	cache_bepinex: false
};

class SettingsRepository {
	async get(): Promise<AppSettings> {
		const raw = await invoke<unknown>('core_get_settings');
		const result = Settings({ ...DEFAULT_SETTINGS, ...(raw as Record<string, unknown>) });
		if (result instanceof type.errors) {
			return DEFAULT_SETTINGS;
		}

		return result;
	}

	async update(updates: Partial<AppSettings>): Promise<void> {
		await invoke('core_update_settings', {
			args: { updates }
		});
	}
}

export const settingsRepository = new SettingsRepository();
