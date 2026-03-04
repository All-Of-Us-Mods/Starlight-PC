import type { QueryClient } from '@tanstack/svelte-query';
import { listen } from '@tauri-apps/api/event';
import { revealItemInDir } from '@tauri-apps/plugin-opener';
import type { AppSettings } from './schema';
import { settingsQueryKey } from './settings-keys';
import type { BepInExProgress } from '../profiles/schema';
import { rustInvoke } from '$lib/infra/rust/invoke';
import { rustMutationOptions } from '$lib/infra/rust/mutation';

type SettingsUpdate = Omit<Partial<AppSettings>, 'xbox_app_id'> & {
	xbox_app_id?: string | null;
};

function normalizeSettingsUpdateForCache(settings: SettingsUpdate): Partial<AppSettings> {
	const { xbox_app_id, ...rest } = settings;
	if (xbox_app_id === null || xbox_app_id === undefined) return rest;
	return { ...rest, xbox_app_id };
}

export const settingsMutations = {
	update: (queryClient: QueryClient) => ({
		mutationFn: (settings: SettingsUpdate) =>
			rustInvoke('core_update_settings', { updates: settings }),
		onSuccess: (updated: AppSettings, variables: SettingsUpdate) => {
			const normalizedVariables = normalizeSettingsUpdateForCache(variables);
			queryClient.setQueryData<AppSettings | undefined>(settingsQueryKey, (current) => {
				if (!current) return updated;
				return { ...current, ...normalizedVariables, ...updated };
			});
		}
	}),
	downloadBepInExToCache: () => ({
		mutationFn: async (args: { url: string; onProgress?: (progress: BepInExProgress) => void }) => {
			let unlisten: (() => void) | undefined;
			try {
				if (args.onProgress) {
					unlisten = await listen<BepInExProgress>('bepinex-progress', (event) =>
						args.onProgress?.(event.payload)
					);
				}
				const cachePath = await rustInvoke('core_get_bepinex_cache_path');
				await rustInvoke('modding_bepinex_cache_download', { url: args.url, cachePath });
			} finally {
				unlisten?.();
			}
		}
	}),
	clearBepInExCache: () => ({
		mutationFn: async () => {
			const cachePath = await rustInvoke('core_get_bepinex_cache_path');
			await rustInvoke('modding_bepinex_cache_clear', { cachePath });
		}
	}),
	autoDetectBepInExArchitecture: (queryClient: QueryClient) => ({
		mutationFn: (gamePath: string) =>
			rustInvoke('core_auto_detect_bepinex_architecture', { gamePath }),
		onSuccess: (updatedUrl: string | null) => {
			if (!updatedUrl) return;
			queryClient.setQueryData<AppSettings | undefined>(settingsQueryKey, (current) =>
				current ? { ...current, bepinex_url: updatedUrl } : current
			);
		}
	}),
	detectAmongUsPath: () => ({
		...rustMutationOptions({
			command: 'platform_detect_among_us'
		})
	}),
	detectGameStore: () => ({
		mutationFn: (path: string) => rustInvoke('platform_detect_game_store', { path })
	}),
	openDataFolder: () => ({
		mutationFn: async () => {
			const appDataPath = await rustInvoke('core_get_app_data_dir');
			await revealItemInDir(appDataPath);
		}
	})
};
