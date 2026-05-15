<script lang="ts" module>
	import type { Snippet } from 'svelte';

	export interface SettingsSectionProps {
		title: string;
		children?: Snippet;
		class?: string;
	}
</script>

<script lang="ts">
	import { cn } from '$lib/utils';

	let { title, children, class: className }: SettingsSectionProps = $props();
</script>

<section
	class={cn('rounded-xl border border-border/50 bg-card/30 p-6 backdrop-blur-sm', className)}
>
	<header class="mb-4">
		<h2 class="text-lg font-semibold tracking-tight">{title}</h2>
	</header>

	{#if children}
		{@render children()}
	{/if}
</section>
