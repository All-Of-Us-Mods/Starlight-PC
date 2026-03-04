<script lang="ts">
	import '../app.css';
	import { QueryClientProvider, createMutation } from '@tanstack/svelte-query';
	import { queryClient } from '$lib/state/query-client';
	import { Toaster } from '$lib/components/ui/sonner';
	import AppShell from '$lib/components/layout/AppShell.svelte';
	import AmongUsPathDialog from '$lib/components/settings/AmongUsPathDialog.svelte';
	import UpdateNotification from '$lib/components/updates/UpdateNotification.svelte';
	import { registerProfilesInvalidateCallback } from '$lib/features/profiles/game-state.svelte';
	import { profileQueries } from '$lib/features/profiles/queries';
	import { settingsQueries } from '$lib/features/settings/queries';
	import { settingsMutations } from '$lib/features/settings/mutations';
	import { watchDirectory } from '$lib/utils/file-watcher';
	import { updateState } from '$lib/features/updates/update-state.svelte';
	import { onMount } from 'svelte';
	import { info, warn } from '@tauri-apps/plugin-log';
	import {
		diskFilesQueryKey,
		profileLogsQueryKey,
		profilesQueryKey,
		unifiedModsQueryKey
	} from '$lib/features/profiles/profile-keys';

	let { children } = $props();
	let dialogOpen = $state(false);
	let detectedPath = $state('');
	const detectAmongUsPathMutation = createMutation(() => settingsMutations.detectAmongUsPath());

	// Register the invalidation callback so game-state can trigger query invalidation
	// without importing queryClient directly
	registerProfilesInvalidateCallback(() => {
		queryClient.invalidateQueries({ queryKey: profilesQueryKey });
	});

	onMount(() => {
		info('Starlight frontend initialized');

		// Check for updates (non-blocking)
		updateState.check();

		let debounceTimer: ReturnType<typeof setTimeout> | undefined;
		let unwatchProfiles: (() => void) | undefined;

		const initializeLayout = async () => {
			const settings = await queryClient.fetchQuery(settingsQueries.get());
			if (!settings.among_us_path) {
				try {
					const path = await detectAmongUsPathMutation.mutateAsync();
					detectedPath = path ?? '';
					dialogOpen = true;
				} catch {
					warn('Failed to auto-detect Among Us path');
					dialogOpen = true;
				}
			}

			// Watch the profiles directory so that metadata.json changes (create, rename,
			// delete, mod updates) are automatically reflected in the UI without relying
			// solely on manual query invalidations.
			// Debounced to avoid rapid-fire invalidations when our own mutations write
			// metadata.json (the mutation already invalidates explicitly).
			try {
				const profilesDir = await queryClient.fetchQuery(profileQueries.dir());
				unwatchProfiles = await watchDirectory(profilesDir, () => {
					clearTimeout(debounceTimer);
					debounceTimer = setTimeout(() => {
						void (async () => {
							try {
								info('Profiles directory changed, invalidating queries');
								// Invalidate profiles list, unified-mods, and disk-files queries
								// since any profile's plugins dir could have changed
								await Promise.all([
									queryClient.invalidateQueries({ queryKey: profilesQueryKey }),
									queryClient.invalidateQueries({ queryKey: unifiedModsQueryKey }),
									queryClient.invalidateQueries({ queryKey: diskFilesQueryKey }),
									queryClient.invalidateQueries({ queryKey: profileLogsQueryKey })
								]);
								info('Profiles, unified-mods, disk-files, and profile-logs queries invalidated');
							} catch (error) {
								warn(`Failed to invalidate profile-related queries: ${error}`);
							}
						})();
					}, 300);
				});
				info(`Watching profiles directory: ${profilesDir}`);
			} catch (err) {
				warn(`Failed to set up profiles directory watcher: ${err}`);
			}
		};
		void initializeLayout().catch((error) => {
			warn(`Failed to initialize layout state: ${error}`);
		});

		// Cleanup function - called when layout is destroyed
		return () => {
			clearTimeout(debounceTimer);
			unwatchProfiles?.();
		};
	});

	document.documentElement.classList.add('dark');
</script>

<QueryClientProvider client={queryClient}>
	<AppShell>
		{@render children()}
	</AppShell>
	<UpdateNotification />
	<Toaster />
	<AmongUsPathDialog bind:open={dialogOpen} {detectedPath} />
</QueryClientProvider>
