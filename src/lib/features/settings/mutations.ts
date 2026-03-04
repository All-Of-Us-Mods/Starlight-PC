import type { QueryClient } from '@tanstack/svelte-query';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { revealItemInDir } from '@tauri-apps/plugin-opener';
import type { AppSettings } from './schema';
import { settingsQueryKey } from './settings-keys';
import type { BepInExProgress } from '../profiles/schema';

export const settingsMutations = {
	update: (queryClient: QueryClient) => ({
		mutationFn: (settings: Partial<AppSettings>) =>
			invoke<AppSettings>('core_update_settings', { args: { updates: settings } }),
		onSuccess: (updated: AppSettings, variables: Partial<AppSettings>) => {
			queryClient.setQueryData<AppSettings | undefined>(settingsQueryKey, (current) => {
				if (!current) return updated;
				return { ...current, ...variables, ...updated };
			});
		}
	}),
	downloadBepInExToCache: () => ({
		mutationFn: async (args: {
			url: string;
			onProgress?: (progress: BepInExProgress) => void;
		}) => {
			let unlisten: (() => void) | undefined;
			try {
				if (args.onProgress) {
					unlisten = await listen<BepInExProgress>('bepinex-progress', (event) =>
						args.onProgress?.(event.payload)
					);
				}
				const cachePath = await invoke<string>('core_get_bepinex_cache_path');
				await invoke<void>('modding_bepinex_cache_download', {
					args: { url: args.url, cachePath }
				});
			} finally {
				unlisten?.();
			}
		}
	}),
	clearBepInExCache: () => ({
		mutationFn: async () => {
			const cachePath = await invoke<string>('core_get_bepinex_cache_path');
			await invoke<void>('modding_bepinex_cache_clear', { args: { cachePath } });
		}
	}),
	autoDetectBepInExArchitecture: (queryClient: QueryClient) => ({
		mutationFn: (gamePath: string) =>
			invoke<string | null>('core_auto_detect_bepinex_architecture', { args: { gamePath } }),
		onSuccess: (updatedUrl: string | null) => {
			if (!updatedUrl) return;
			queryClient.setQueryData<AppSettings | undefined>(settingsQueryKey, (current) =>
				current ? { ...current, bepinex_url: updatedUrl } : current
			);
		}
	}),
	openDataFolder: () => ({
		mutationFn: async () => {
			const appDataPath = await invoke<string>('core_get_app_data_dir');
			await revealItemInDir(appDataPath);
		}
	}),
	checkCacheExists: () => ({
		mutationFn: async () => {
			const cachePath = await invoke<string>('core_get_bepinex_cache_path');
			return await invoke<boolean>('modding_bepinex_cache_exists', { args: { cachePath } });
		}
	})
};
