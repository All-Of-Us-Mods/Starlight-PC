<script lang="ts">
	import '../app.css';
	import { PersistQueryClientProvider } from '@tanstack/svelte-query-persist-client';
	import { queryClient, tauriPersister } from '$lib/state/queryClient';
	import { Toaster } from '$lib/components/ui/sonner';
	import AppShell from '$lib/components/layout/AppShell.svelte';
	import AmongUsPathDialog from '$lib/features/settings/components/AmongUsPathDialog.svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { settingsService } from '$lib/features/settings/settings-service';
	import { onMount } from 'svelte';
	import { info, warn } from '@tauri-apps/plugin-log';

	let { children } = $props();
	let dialogOpen = $state(false);
	let detectedPath = $state('');

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

<PersistQueryClientProvider client={queryClient} persistOptions={{ persister: tauriPersister }}>
	<AppShell>
		{@render children()}
	</AppShell>
</PersistQueryClientProvider>
<Toaster />
<AmongUsPathDialog bind:open={dialogOpen} {detectedPath} />
