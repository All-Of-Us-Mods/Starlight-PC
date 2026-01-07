import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { settingsService } from '$lib/features/settings/settings-service';

export interface BepInExProgress {
	stage: 'downloading' | 'extracting' | 'complete';
	progress: number;
	message: string;
}

export async function downloadBepInEx(
	profilePath: string,
	bepinexUrl: string,
	onProgress?: (progress: BepInExProgress) => void
): Promise<void> {
	let unlisten: UnlistenFn | undefined;

	try {
		if (onProgress) {
			unlisten = await listen<BepInExProgress>('bepinex-progress', (event) => {
				onProgress(event.payload);
			});
		}

		const settings = await settingsService.getSettings();
		const cachePath = settings.cache_bepinex ? await settingsService.getBepInExCachePath() : null;

		await invoke('install_bepinex', {
			url: bepinexUrl,
			destination: profilePath,
			cachePath
		});
	} finally {
		unlisten?.();
	}
}
