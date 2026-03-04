import { invoke } from '@tauri-apps/api/core';
import { revealItemInDir } from '@tauri-apps/plugin-opener';
import { settingsRepository } from './settings-repository';
import type { BepInExProgress } from '../profiles/schema';
import type { GamePlatform } from './schema';

class SettingsService {
	readonly getSettings = () => settingsRepository.get();
	readonly updateSettings = (updates: Parameters<typeof settingsRepository.update>[0]) =>
		settingsRepository.update(updates);

	readonly detectAmongUsPath = () => invoke<string | null>('platform_detect_among_us');
	readonly detectGamePlatform = (path: string) =>
		invoke<GamePlatform>('platform_detect_game_store', { args: { path } });

	async getBepInExCachePath(): Promise<string> {
		return invoke<string>('core_get_bepinex_cache_path');
	}

	async checkBepInExCacheExists(): Promise<boolean> {
		return invoke<boolean>('modding_bepinex_cache_exists', {
			args: { cachePath: await this.getBepInExCachePath() }
		});
	}

	async downloadBepInExToCache(url: string, onProgress?: (progress: BepInExProgress) => void) {
		const { listen } = await import('@tauri-apps/api/event');
		let unlisten: (() => void) | undefined;
		try {
			if (onProgress) {
				unlisten = await listen<BepInExProgress>('bepinex-progress', (e) => onProgress(e.payload));
			}
			await invoke('modding_bepinex_cache_download', {
				args: { url, cachePath: await this.getBepInExCachePath() }
			});
		} finally {
			unlisten?.();
		}
	}

	async clearBepInExCache() {
		await invoke('modding_bepinex_cache_clear', {
			args: { cachePath: await this.getBepInExCachePath() }
		});
	}

	async openDataFolder() {
		await revealItemInDir(await invoke<string>('core_get_app_data_dir'));
	}

	/**
	 * Auto-detects the game architecture (x86/x64) and updates the BepInEx URL accordingly.
	 * Checks for UnityCrashHandler64.exe to determine if the game is 64-bit.
	 * @returns The new URL if it was updated, undefined otherwise.
	 */
	async autoDetectBepInExArchitecture(gamePath: string): Promise<string | undefined> {
		const result = await invoke<string | null>('core_auto_detect_bepinex_architecture', {
			args: { gamePath }
		});
		return result ?? undefined;
	}
}

export const settingsService = new SettingsService();
