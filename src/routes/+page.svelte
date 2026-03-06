<script lang="ts">
	import { createQuery } from '@tanstack/svelte-query';
	import { newsQueries } from '$lib/features/news/queries';
	import { modQueries } from '$lib/features/mods/queries';
	import type { Post } from '$lib/features/news/schema';
	import { getSidebar } from '$lib/features/app/state/sidebar.svelte';

	import NewsDetail from '$lib/features/news/components/NewsDetail.svelte';
	import HomeNewsSection from '$lib/features/home/components/HomeNewsSection.svelte';
	import HomeTrendingModsSection from '$lib/features/home/components/HomeTrendingModsSection.svelte';

	const newsQuery = createQuery(newsQueries.all);
	const trendingModsQuery = createQuery(modQueries.trending);

	const sidebar = getSidebar();

	let selectedPost = $state<Post | null>(null);

	function toggleNews(item: Post) {
		const opened = sidebar.open(NewsDetailSidebar, () => (selectedPost = null), `news-${item.id}`);
		selectedPost = opened ? item : null;
	}

	function closeSidebar() {
		selectedPost = null;
		sidebar.close();
	}
</script>

{#snippet NewsDetailSidebar()}
	{#if selectedPost}
		<NewsDetail post={selectedPost} onclose={closeSidebar} />
	{/if}
{/snippet}

<div class="scrollbar-styled @container h-full space-y-12 overflow-y-auto px-10 py-8">
	<HomeNewsSection
		{newsQuery}
		selectedPostId={selectedPost?.id ?? null}
		onToggleNews={toggleNews}
	/>
	<HomeTrendingModsSection {trendingModsQuery} />
</div>
