import type { QueryClient } from '@tanstack/svelte-query';
import { startupState } from '../state/startup.svelte';
import { profileQueries } from '$lib/features/profiles/queries';
import { settingsQueries } from '$lib/features/settings/queries';
import { rustInvoke } from '$lib/infra/rust/invoke';
import { updateState } from '$lib/features/updates/state/update-state.svelte';
import { watchProfileDirectory } from './profile-directory-watch';
import { info, warn } from '@tauri-apps/plugin-log';

export async function bootstrapApp(queryClient: QueryClient): Promise<() => void> {
	await info('Starlight frontend initialized');

	void updateState.check();

	const settings = await queryClient.fetchQuery(settingsQueries.get());
	if (!settings.among_us_path) {
		try {
			const path = await rustInvoke('platform_detect_among_us');
			startupState.showAmongUsPathDialog(path ?? '');
		} catch {
			await warn('Failed to auto-detect Among Us path');
			startupState.showAmongUsPathDialog();
		}
	}

	try {
		const profilesDir = await queryClient.fetchQuery(profileQueries.dir());
		return watchProfileDirectory(queryClient, profilesDir);
	} catch (error) {
		await warn(`Failed to initialize bootstrap state: ${error}`);
		return () => {};
	}
}
