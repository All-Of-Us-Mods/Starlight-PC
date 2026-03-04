import { invoke } from '@tauri-apps/api/core';
import { settingsService } from '../settings/settings-service';
import { gameState } from './game-state.svelte';
import { info, error as logError, debug } from '@tauri-apps/plugin-log';
import type { Profile } from './schema';

class LaunchService {
	private launchInFlight = false;

	async launchProfile(profile: Profile): Promise<void> {
		if (this.launchInFlight) throw new Error('A launch is already in progress');
		this.launchInFlight = true;
		try {
			info(`Launching profile: ${profile.name} (${profile.id})`);
			const settings = await settingsService.getSettings();

			if (!settings.among_us_path) {
				logError('Among Us path not configured');
				throw new Error('Among Us path not configured');
			}
			if (!settings.allow_multi_instance_launch && gameState.running) {
				throw new Error('An Among Us instance is already running');
			}

			debug('Invoking game_launch_profile workflow command');
			const result = await invoke<{ close_on_launch: boolean }>('game_launch_profile', {
				args: { profileId: profile.id, profilePath: profile.path }
			});
			info(`Profile ${profile.name} launched successfully`);

			if (result.close_on_launch) {
				debug('Closing window on launch');
				const { getCurrentWindow } = await import('@tauri-apps/api/window');
				getCurrentWindow().close();
			}
		} finally {
			this.launchInFlight = false;
		}
	}

	async launchVanilla(): Promise<void> {
		if (this.launchInFlight) throw new Error('A launch is already in progress');
		this.launchInFlight = true;
		try {
			info('Launching vanilla Among Us');
			const settings = await settingsService.getSettings();

			if (!settings.among_us_path) {
				logError('Among Us path not configured');
				throw new Error('Among Us path not configured');
			}
			if (!settings.allow_multi_instance_launch && gameState.running) {
				throw new Error('An Among Us instance is already running');
			}

			debug('Invoking game_launch_vanilla_workflow command');
			await invoke<{ close_on_launch: boolean }>('game_launch_vanilla_workflow');
			info('Vanilla game launched successfully');
		} finally {
			this.launchInFlight = false;
		}
	}
}

export const launchService = new LaunchService();
