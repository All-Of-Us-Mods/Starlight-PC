<script lang="ts" module>
	import type { Mod } from '$lib/features/mods/schema';

	export interface HomeTrendingModsSectionProps {
		trendingModsQuery: {
			isLoading: boolean;
			isError: boolean;
			isSuccess: boolean;
			data?: Mod[];
		};
	}
</script>

<script lang="ts">
	import * as Carousel from '$lib/components/ui/carousel';
	import { TrendingUp } from '@lucide/svelte';
	import ModCard from '$lib/features/mods/components/ModCard.svelte';
	import ModCardSkeleton from '$lib/features/mods/components/ModCardSkeleton.svelte';

	let { trendingModsQuery }: HomeTrendingModsSectionProps = $props();
</script>

<section>
	<div class="mb-6 flex items-center gap-3">
		<div
			class="flex h-12 w-12 items-center justify-center rounded-xl bg-primary/10 ring-1 ring-primary/20"
		>
			<TrendingUp class="h-6 w-6 text-primary" />
		</div>
		<div class="space-y-0.5">
			<h2 class="text-4xl font-black tracking-tight">Trending Mods</h2>
			<p class="text-sm text-muted-foreground">Most popular community creations this week.</p>
		</div>
	</div>

	<Carousel.Root opts={{ align: 'start' }} class="w-full">
		<Carousel.Content class="-ml-2">
			{#if trendingModsQuery.isLoading}
				{#each { length: 6 }, i (i)}
					<Carousel.Item class="basis-full pl-2 @4xl:basis-1/2 @7xl:basis-1/3">
						<ModCardSkeleton />
					</Carousel.Item>
				{/each}
			{:else if trendingModsQuery.isError}
				<Carousel.Item class="basis-full pl-2">
					<div class="rounded-md border border-destructive/20 bg-destructive/10 p-4">
						<p class="text-sm font-semibold text-destructive">Error loading mods</p>
					</div>
				</Carousel.Item>
			{:else if trendingModsQuery.isSuccess && trendingModsQuery.data}
				{#each trendingModsQuery.data as mod (mod.id)}
					<Carousel.Item class="basis-full pl-2 select-none @4xl:basis-1/2 @7xl:basis-1/3">
						<ModCard {mod} />
					</Carousel.Item>
				{/each}
			{/if}
		</Carousel.Content>
		<Carousel.Previous class="-left-9" />
		<Carousel.Next class="-right-9" />
	</Carousel.Root>
</section>
