import type { QueryClient } from '@tanstack/svelte-query';
import { watchDirectory } from '$lib/infra/tauri/fs-watch';
import { info, warn } from '@tauri-apps/plugin-log';
import {
	diskFilesQueryKey,
	profileLogsQueryKey,
	profilesQueryKey,
	unifiedModsQueryKey
} from '$lib/features/profiles/profile-keys';

export async function watchProfileDirectory(
	queryClient: QueryClient,
	profilesDir: string
): Promise<() => void> {
	let debounceTimer: ReturnType<typeof setTimeout> | undefined;

	try {
		const unwatchProfiles = await watchDirectory(profilesDir, () => {
			clearTimeout(debounceTimer);
			debounceTimer = setTimeout(() => {
				void (async () => {
					try {
						await info('Profiles directory changed, invalidating queries');
						await Promise.all([
							queryClient.invalidateQueries({ queryKey: profilesQueryKey }),
							queryClient.invalidateQueries({ queryKey: unifiedModsQueryKey }),
							queryClient.invalidateQueries({ queryKey: diskFilesQueryKey }),
							queryClient.invalidateQueries({ queryKey: profileLogsQueryKey })
						]);
						await info('Profiles, unified-mods, disk-files, and profile-logs queries invalidated');
					} catch (error) {
						await warn(`Failed to invalidate profile-related queries: ${error}`);
					}
				})();
			}, 300);
		});

		await info(`Watching profiles directory: ${profilesDir}`);
		return () => {
			clearTimeout(debounceTimer);
			unwatchProfiles();
		};
	} catch (error) {
		await warn(`Failed to set up profiles directory watcher: ${error}`);
		return () => {
			clearTimeout(debounceTimer);
		};
	}
}
