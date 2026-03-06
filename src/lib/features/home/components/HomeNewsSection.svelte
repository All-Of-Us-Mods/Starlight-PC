<script lang="ts" module>
	import type { Post } from '$lib/features/news/schema';

	export interface HomeNewsSectionProps {
		newsQuery: {
			isLoading: boolean;
			isError: boolean;
			isSuccess: boolean;
			data?: Post[];
		};
		selectedPostId?: number | null;
		onToggleNews: (post: Post) => void;
	}
</script>

<script lang="ts">
	import * as Carousel from '$lib/components/ui/carousel';
	import { Newspaper } from '@lucide/svelte';
	import NewsCard from '$lib/features/news/components/NewsCard.svelte';
	import NewsCardSkeleton from '$lib/features/news/components/NewsCardSkeleton.svelte';

	let { newsQuery, selectedPostId = null, onToggleNews }: HomeNewsSectionProps = $props();
</script>

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

	<Carousel.Root opts={{ align: 'start' }} class="w-full">
		<Carousel.Content class="-ml-4">
			{#if newsQuery.isLoading}
				{#each { length: 6 }, i (i)}
					<Carousel.Item class="pl-4 @lg:basis-1/2 @2xl:basis-1/3">
						<NewsCardSkeleton />
					</Carousel.Item>
				{/each}
			{:else if newsQuery.isError}
				<Carousel.Item class="basis-full pl-4">
					<div class="rounded-md border border-destructive/20 bg-destructive/10 p-4">
						<p class="text-sm font-semibold text-destructive">Error loading posts</p>
					</div>
				</Carousel.Item>
			{:else if newsQuery.isSuccess && newsQuery.data}
				{#each newsQuery.data as post (post.id)}
					<Carousel.Item class="pl-4 select-none @lg:basis-1/2 @2xl:basis-1/3">
						<NewsCard
							{post}
							isSelected={selectedPostId === post.id}
							onclick={() => onToggleNews(post)}
						/>
					</Carousel.Item>
				{/each}
			{/if}
		</Carousel.Content>
		<Carousel.Previous class="-left-9" />
		<Carousel.Next class="-right-9" />
	</Carousel.Root>
</section>
