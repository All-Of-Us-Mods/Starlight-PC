import { invoke } from '@tauri-apps/api/core';
import { join } from '@tauri-apps/api/path';
import { exists } from '@tauri-apps/plugin-fs';
import { profileService } from './profile-service';
import { settingsService } from '../settings/settings-service';
import { gameState } from './game-state-service.svelte';
import { epicService } from '$lib/features/settings/epic-service';
import { info, error as logError, debug } from '@tauri-apps/plugin-log';
import type { Profile } from './schema';

class LaunchService {
	async launchProfile(profile: Profile): Promise<void> {
		info(`Launching profile: ${profile.name} (${profile.id})`);
		const settings = await settingsService.getSettings();

		if (!settings.among_us_path) {
			logError('Among Us path not configured');
			throw new Error('Among Us path not configured');
		}

		const gameExePath = await join(settings.among_us_path, 'Among Us.exe');
		const gameExists = await exists(gameExePath);
		if (!gameExists) {
			logError(`Among Us.exe not found at: ${gameExePath}`);
			throw new Error('Among Us.exe not found at configured path');
		}

		const bepinexDll = await join(profile.path, 'BepInEx', 'core', 'BepInEx.Unity.IL2CPP.dll');
		const bepinexExists = await exists(bepinexDll);
		if (!bepinexExists) {
			logError(`BepInEx DLL not found at: ${bepinexDll}`);
			throw new Error('BepInEx DLL not found. Please wait for installation to complete.');
		}

		const dotnetDir = await join(profile.path, 'dotnet');
		const coreClr = await join(dotnetDir, 'coreclr.dll');
		const coreClrExists = await exists(coreClr);
		if (!coreClrExists) {
			logError(`dotnet runtime not found at: ${coreClr}`);
			throw new Error('dotnet runtime not found. Please wait for installation to complete.');
		}

		if (settings.game_platform === 'epic') {
			debug('Epic platform detected, ensuring logged in');
			await epicService.ensureLoggedIn();
		}

		debug('Invoking launch_modded command');
		await invoke('launch_modded', {
			gameExe: gameExePath,
			profilePath: profile.path,
			bepinexDll: bepinexDll,
			dotnetDir: dotnetDir,
			coreclrPath: coreClr,
			platform: settings.game_platform || 'steam'
		});

		await profileService.updateLastLaunched(profile.id);
		gameState.setRunningProfile(profile.id);
		info(`Profile ${profile.name} launched successfully`);

		if (settings.close_on_launch) {
			debug('Closing window on launch');
			const { getCurrentWindow } = await import('@tauri-apps/api/window');
			getCurrentWindow().close();
		}
	}

	async launchVanilla(): Promise<void> {
		info('Launching vanilla Among Us');
		const settings = await settingsService.getSettings();

		if (!settings.among_us_path) {
			logError('Among Us path not configured');
			throw new Error('Among Us path not configured');
		}

		const gameExePath = await join(settings.among_us_path, 'Among Us.exe');
		const gameExists = await exists(gameExePath);
		if (!gameExists) {
			logError(`Among Us.exe not found at: ${gameExePath}`);
			throw new Error('Among Us.exe not found at configured path');
		}

		debug('Invoking launch_vanilla command');
		await invoke('launch_vanilla', {
			gameExe: gameExePath,
			platform: settings.game_platform || 'steam'
		});
		gameState.setRunningProfile(null);
		info('Vanilla game launched successfully');
	}
}

export const launchService = new LaunchService();
