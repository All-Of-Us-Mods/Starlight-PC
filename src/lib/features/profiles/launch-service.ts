import { invoke } from '@tauri-apps/api/core';
import { Command } from '@tauri-apps/plugin-shell';
import { Store } from '@tauri-apps/plugin-store';
import { join } from '@tauri-apps/api/path';
import { profileService } from './profile-service';
import type { Profile } from './schema';
import type { AppSettings } from '../settings/schema';

class LaunchService {
	private async getSettings(): Promise<AppSettings> {
		const store = await Store.load('registry.json');
		return (
			(await store.get<AppSettings>('settings')) ?? {
				bepinex_url: '',
				bepinex_version: '',
				among_us_path: '',
				close_on_launch: false
			}
		);
	}

	async launchProfile(profile: Profile): Promise<void> {
		const settings = await this.getSettings();

		if (!settings.among_us_path) throw new Error('Among Us path not configured');

		const isRunning = await invoke<boolean>('check_among_us_running');
		if (isRunning) throw new Error('Among Us is already running');

		await invoke('set_dll_directory', { profilePath: profile.path });

		const bepinexDll = await join(profile.path, 'BepInEx', 'core', 'BepInEx.Unity.IL2CPP.dll');
		const dotnetDir = await join(profile.path, 'dotnet');
		const coreClr = await join(dotnetDir, 'coreclr.dll');

		const command = Command.create(
			'launch-among-us',
			[
				'/c',
				'Among Us.exe',
				'--doorstop-enabled',
				'true',
				'--doorstop-target-assembly',
				bepinexDll,
				'--doorstop-clr-corlib-dir',
				dotnetDir,
				'--doorstop-clr-runtime-coreclr-path',
				coreClr
			],
			{ cwd: settings.among_us_path }
		);

		await command.spawn();
		await profileService.updateLastLaunched(profile.id);

		if (settings.close_on_launch) {
			const { getCurrentWindow } = await import('@tauri-apps/api/window');
			getCurrentWindow().close();
		}
	}

	async launchVanilla(): Promise<void> {
		const settings = await this.getSettings();
		const command = Command.create('launch-among-us', ['/c', 'Among Us.exe'], {
			cwd: settings.among_us_path
		});

		await command.spawn();
	}
}

export const launchService = new LaunchService();
