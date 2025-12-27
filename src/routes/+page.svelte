<script lang="ts">
	import { createQuery } from '@tanstack/svelte-query';
	import { newsQueries } from '$lib/features/news/queries';
	import { modQueries } from '$lib/features/mods/queries';
	import type { Post } from '$lib/features/news/schema';
	import * as Carousel from '$lib/components/ui/carousel';
	import { getSidebar } from '$lib/state/sidebar.svelte';
	import { Newspaper, TrendingUp } from '@lucide/svelte';

	import NewsCard from '$lib/features/news/components/NewsCard.svelte';
	import ModCard from '$lib/features/mods/components/ModCard.svelte';
	import NewsDetail from '$lib/features/news/components/NewsDetail.svelte';
	import NewsCardSkeleton from '$lib/features/news/components/NewsCardSkeleton.svelte';
	import ModCardSkeleton from '$lib/features/mods/components/ModCardSkeleton.svelte';

	const newsQuery = createQuery(newsQueries.all);
	const trendingModsQuery = createQuery(modQueries.trending);

	const sidebar = getSidebar();

	let selectedPost = $state<Post | null>(null);
	let displayedPost = $state<Post | null>(null);

	function toggleNews(item: Post) {
		if (selectedPost?.id === item.id) {
			selectedPost = null;
			sidebar.close();
		} else {
			selectedPost = item;
			displayedPost = item;
			sidebar.open(NewsDetailSidebar, () => {
				displayedPost = null;
			});
		}
	}

	function closeSidebar() {
		selectedPost = null;
		sidebar.close();
	}
</script>

{#snippet NewsDetailSidebar()}
	{#if displayedPost}
		<NewsDetail post={displayedPost} onclose={closeSidebar} />
	{/if}
{/snippet}

<div class="scrollbar-styled @container h-full space-y-12 overflow-y-auto px-10 py-8">
	<!-- News Section -->
	<section>
		<div class="mb-6 flex items-center gap-3">
			<div
				class="flex h-12 w-12 items-center justify-center rounded-xl bg-primary/10 ring-1 ring-primary/20"
			>
				<Newspaper class="h-6 w-6 text-primary" />
			</div>
			<div class="space-y-0.5">
				<h2 class="text-4xl font-black tracking-tight">Latest News</h2>
				<p class="text-sm text-muted-foreground">Updates from the development team.</p>
			</div>
		</div>

		{#if newsQuery.isLoading}
			<Carousel.Root opts={{ align: 'start' }} class="w-full">
				<Carousel.Content class="-ml-4">
					{#each { length: 6 }, i (i)}
						<Carousel.Item class="pl-4 @lg:basis-1/2 @2xl:basis-1/3">
							<NewsCardSkeleton />
						</Carousel.Item>
					{/each}
				</Carousel.Content>
				<Carousel.Previous class="-left-9" />
				<Carousel.Next class="-right-9" />
			</Carousel.Root>
		{:else if newsQuery.isError}
			<div class="mx-10 rounded-md border border-destructive/20 bg-destructive/10 p-4">
				<p class="text-sm font-semibold text-destructive">Error loading posts</p>
			</div>
		{:else if newsQuery.isSuccess && newsQuery.data}
			<Carousel.Root opts={{ align: 'start' }} class="w-full">
				<Carousel.Content class="-ml-4">
					{#each newsQuery.data as post (post.id)}
						<Carousel.Item class="pl-4 @lg:basis-1/2 @2xl:basis-1/3">
							<NewsCard
								{post}
								isSelected={selectedPost?.id === post.id}
								onclick={() => toggleNews(post)}
							/>
						</Carousel.Item>
					{/each}
				</Carousel.Content>
				<Carousel.Previous class="-left-9" />
				<Carousel.Next class="-right-9" />
			</Carousel.Root>
		{/if}
	</section>

	<!-- Trending Mods Section -->
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

		{#if trendingModsQuery.isLoading}
			<Carousel.Root opts={{ align: 'start' }} class="w-full">
				<Carousel.Content class="-ml-2">
					{#each { length: 6 }, i (i)}
						<Carousel.Item class="basis-full pl-2 @4xl:basis-1/2 @7xl:basis-1/3">
							<ModCardSkeleton />
						</Carousel.Item>
					{/each}
				</Carousel.Content>
				<Carousel.Previous class="-left-9" />
				<Carousel.Next class="-right-9" />
			</Carousel.Root>
		{:else if trendingModsQuery.isError}
			<div class="rounded-md border border-destructive/20 bg-destructive/10 p-4">
				<p class="text-sm font-semibold text-destructive">Error loading mods</p>
			</div>
		{:else if trendingModsQuery.isSuccess && trendingModsQuery.data}
			<Carousel.Root opts={{ align: 'start' }} class="w-full">
				<Carousel.Content class="-ml-2">
					{#each trendingModsQuery.data as mod (mod.id)}
						<Carousel.Item class="basis-full pl-2 @4xl:basis-1/2 @7xl:basis-1/3">
							<ModCard {mod} />
						</Carousel.Item>
					{/each}
				</Carousel.Content>
				<Carousel.Previous class="-left-9" />
				<Carousel.Next class="-right-9" />
			</Carousel.Root>
		{/if}
	</section>
</div>
