<script lang="ts" module>
	import type { Component } from 'svelte';
	import type { Snippet } from 'svelte';

	export interface PageHeaderProps {
		title: string;
		description: string;
		icon: Component<{ class?: string }>;
		class?: string;
		children?: Snippet;
	}
</script>

<script lang="ts">
	import { cn } from '$lib/utils';

	let { title, description, icon: Icon, class: className, children }: PageHeaderProps = $props();
</script>

<header class={cn('mb-6 flex items-center justify-between gap-3', className)}>
	<div class="flex items-center gap-3">
		<div
			class="flex h-12 w-12 items-center justify-center rounded-xl bg-primary/10 ring-1 ring-primary/20"
		>
			<Icon class="h-6 w-6 text-primary" />
		</div>
		<div class="space-y-0.5">
			<h1 class="text-4xl font-black tracking-tight">{title}</h1>
			<p class="text-sm text-muted-foreground">{description}</p>
		</div>
	</div>
	{#if children}
		{@render children()}
	{/if}
</header>
