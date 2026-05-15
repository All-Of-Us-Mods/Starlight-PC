<script lang="ts">
	import '../app.css';
	import { QueryClientProvider } from '@tanstack/svelte-query';
	import { queryClient } from '$lib/infra/query/client';
	import { Toaster } from '$lib/components/ui/sonner';
	import AppShellContainer from '$lib/features/app/containers/AppShellContainer.svelte';
	import AppBootstrap from '$lib/features/app/containers/AppBootstrap.svelte';
	import AmongUsPathDialogContainer from '$lib/features/settings/containers/AmongUsPathDialogContainer.svelte';
	import UpdateNotificationContainer from '$lib/features/updates/containers/UpdateNotificationContainer.svelte';

	let { children } = $props();

	document.documentElement.classList.add('dark');
</script>

<QueryClientProvider client={queryClient}>
	<AppBootstrap />
	<AppShellContainer>
		{@render children()}
	</AppShellContainer>
	<UpdateNotificationContainer />
	<Toaster />
	<AmongUsPathDialogContainer />
</QueryClientProvider>
