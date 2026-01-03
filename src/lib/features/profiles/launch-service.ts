import { invoke } from '@tauri-apps/api/core';
import { join } from '@tauri-apps/api/path';
import { exists } from '@tauri-apps/plugin-fs';
import { profileService } from './profile-service';
import { settingsService } from '../settings/settings-service';
import { gameState } from './game-state-service.svelte';
import { epicService } from '$lib/features/settings/epic-service';
import type { Profile } from './schema';

class LaunchService {
	async launchProfile(profile: Profile): Promise<void> {
		const settings = await settingsService.getSettings();

		if (!settings.among_us_path) throw new Error('Among Us path not configured');

		const gameExePath = await join(settings.among_us_path, 'Among Us.exe');
		const gameExists = await exists(gameExePath);
		if (!gameExists) {
			throw new Error('Among Us.exe not found at configured path');
		}

		const bepinexDll = await join(profile.path, 'BepInEx', 'core', 'BepInEx.Unity.IL2CPP.dll');
		const bepinexExists = await exists(bepinexDll);
		if (!bepinexExists) {
			throw new Error('BepInEx DLL not found. Please wait for installation to complete.');
		}

		const dotnetDir = await join(profile.path, 'dotnet');
		const coreClr = await join(dotnetDir, 'coreclr.dll');
		const coreClrExists = await exists(coreClr);
		if (!coreClrExists) {
			throw new Error('dotnet runtime not found. Please wait for installation to complete.');
		}

		if (settings.game_platform === 'epic') {
			await epicService.ensureLoggedIn();
		}

		await invoke('save_game_copy', { path: settings.among_us_path });
		await invoke('launch_modded', {
			gameExe: gameExePath,
			profilePath: profile.path,
			bepinexDll: bepinexDll,
			dotnetDir: dotnetDir,
			coreclrPath: coreClr
		});

		await profileService.updateLastLaunched(profile.id);
		gameState.setRunningProfile(profile.id);

		if (settings.close_on_launch) {
			const { getCurrentWindow } = await import('@tauri-apps/api/window');
			getCurrentWindow().close();
		}
	}

	async launchVanilla(): Promise<void> {
		const settings = await settingsService.getSettings();

		if (!settings.among_us_path) throw new Error('Among Us path not configured');

		const gameExePath = await join(settings.among_us_path, 'Among Us.exe');
		const gameExists = await exists(gameExePath);
		if (!gameExists) {
			throw new Error('Among Us.exe not found at configured path');
		}

		await invoke('save_game_copy', { path: settings.among_us_path });
		await invoke('launch_vanilla', { gameExe: gameExePath });
		gameState.setRunningProfile(null);
	}
}

export const launchService = new LaunchService();
