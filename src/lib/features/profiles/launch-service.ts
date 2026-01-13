import { invoke } from '@tauri-apps/api/core';
import { join } from '@tauri-apps/api/path';
import { exists } from '@tauri-apps/plugin-fs';
import { settingsService } from '../settings/settings-service';
import { gameState } from './game-state.svelte';
import { epicService } from '$lib/features/settings/epic-service';
import { info, error as logError, debug } from '@tauri-apps/plugin-log';
import type { Profile } from './schema';

class LaunchService {
	/**
	 * Gets or fetches the Xbox AppUserModelId, caching it in settings.
	 */
	private async getXboxAppId(): Promise<string> {
		const settings = await settingsService.getSettings();

		if (settings.xbox_app_id) {
			debug(`Using cached Xbox AppUserModelId: ${settings.xbox_app_id}`);
			return settings.xbox_app_id;
		}

		info('Fetching Xbox AppUserModelId...');
		const appId = await invoke<string>('get_xbox_app_id');
		await settingsService.updateSettings({ xbox_app_id: appId });
		return appId;
	}

	async launchProfile(profile: Profile): Promise<void> {
		info(`Launching profile: ${profile.name} (${profile.id})`);
		const settings = await settingsService.getSettings();

		if (!settings.among_us_path) {
			logError('Among Us path not configured');
			throw new Error('Among Us path not configured');
		}

		// Xbox platform uses a different launch flow
		if (settings.game_platform === 'xbox') {
			await this.launchXboxModded(profile, settings.among_us_path);
			return;
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

		gameState.setRunningProfile(profile.id);
		info(`Profile ${profile.name} launched successfully`);

		if (settings.close_on_launch) {
			debug('Closing window on launch');
			const { getCurrentWindow } = await import('@tauri-apps/api/window');
			getCurrentWindow().close();
		}
	}

	/**
	 * Launches a modded profile for Xbox/Microsoft Store platform.
	 */
	private async launchXboxModded(profile: Profile, gamePath: string): Promise<void> {
		debug('Xbox platform detected, using Xbox launch flow');

		// Get or fetch the AppUserModelId
		const appId = await this.getXboxAppId();

		// Prepare launch (copy files, modify INI)
		info('Preparing Xbox launch...');
		await invoke('prepare_xbox_launch', {
			gameDir: gamePath,
			profilePath: profile.path
		});

		// Launch the game
		info('Launching Xbox game...');
		await invoke('launch_xbox', { appId });

		gameState.setRunningProfile(profile.id);
		info(`Profile ${profile.name} launched successfully (Xbox)`);

		const settings = await settingsService.getSettings();
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

		// Xbox platform uses a different launch flow
		if (settings.game_platform === 'xbox') {
			await this.launchXboxVanilla(settings.among_us_path);
			return;
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

	/**
	 * Launches vanilla Among Us for Xbox/Microsoft Store platform.
	 */
	private async launchXboxVanilla(gamePath: string): Promise<void> {
		debug('Xbox platform detected, using Xbox vanilla launch flow');

		// Get or fetch the AppUserModelId
		const appId = await this.getXboxAppId();

		// Clean up any modding files first
		info('Cleaning up Xbox modding files...');
		await invoke('cleanup_xbox_files', { gameDir: gamePath });

		// Launch the game
		info('Launching Xbox game (vanilla)...');
		await invoke('launch_xbox', { appId });

		gameState.setRunningProfile(null);
		info('Vanilla game launched successfully (Xbox)');
	}
}

export const launchService = new LaunchService();
