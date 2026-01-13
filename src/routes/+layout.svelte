<script lang="ts">
	import '../app.css';
	import { QueryClientProvider } from '@tanstack/svelte-query';
	import { queryClient } from '$lib/state/query-client';
	import { Toaster } from '$lib/components/ui/sonner';
	import AppShell from '$lib/components/layout/AppShell.svelte';
	import AmongUsPathDialog from '$lib/features/settings/AmongUsPathDialog.svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { settingsService } from '$lib/features/settings/settings-service';
	import { registerProfilesInvalidateCallback } from '$lib/features/profiles/game-state.svelte';
	import { onMount } from 'svelte';
	import { info, warn } from '@tauri-apps/plugin-log';

	let { children } = $props();
	let dialogOpen = $state(false);
	let detectedPath = $state('');

	// Register the invalidation callback so game-state can trigger query invalidation
	// without importing queryClient directly
	registerProfilesInvalidateCallback(() => {
		queryClient.invalidateQueries({ queryKey: ['profiles'] });
	});

	onMount(async () => {
		info('Starlight frontend initialized');

		const settings = await settingsService.getSettings();
		if (!settings.among_us_path) {
			try {
				const path = await invoke<string | null>('detect_among_us');
				detectedPath = path ?? '';
				dialogOpen = true;
			} catch {
				warn('Failed to auto-detect Among Us path');
				dialogOpen = true;
			}
		}
	});

	document.documentElement.classList.add('dark');
</script>

<QueryClientProvider client={queryClient}>
	<AppShell>
		{@render children()}
	</AppShell>
	<Toaster />
	<AmongUsPathDialog bind:open={dialogOpen} {detectedPath} />
</QueryClientProvider>
