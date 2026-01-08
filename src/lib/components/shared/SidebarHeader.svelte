<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { X, Maximize, Minimize } from '@jis3r/icons';
	import { getSidebar } from '$lib/state/sidebar.svelte';

	interface Props {
		onclose?: () => void;
	}

	let { onclose }: Props = $props();

	const sidebar = getSidebar();
</script>

<header
	class="sticky top-0 z-10 flex items-center justify-end gap-1.5 border-b border-border/50 bg-muted/90 px-3 py-2 backdrop-blur-sm"
>
	<Button variant="ghost" size="icon-sm" onclick={() => sidebar.toggleMaximize()}>
		{#if sidebar.isMaximized}
			<Minimize class="size-4" />
		{:else}
			<Maximize class="size-4" />
		{/if}
	</Button>
	<Button variant="ghost" size="icon-sm" onclick={onclose}>
		<X class="size-4" />
	</Button>
</header>
