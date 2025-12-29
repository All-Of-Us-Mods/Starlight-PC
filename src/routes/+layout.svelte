<script lang="ts">
	import '../app.css';
	import { PersistQueryClientProvider } from '@tanstack/svelte-query-persist-client';
	import { queryClient, tauriPersister } from '$lib/state/queryClient';
	import { Toaster } from '$lib/components/ui/sonner';
	import AppShell from '$lib/components/layout/AppShell.svelte';

	let { children } = $props();

	document.documentElement.classList.add('dark');
</script>

<PersistQueryClientProvider client={queryClient} persistOptions={{ persister: tauriPersister }}>
	<AppShell>
		{@render children()}
	</AppShell>
</PersistQueryClientProvider>
<Toaster />
