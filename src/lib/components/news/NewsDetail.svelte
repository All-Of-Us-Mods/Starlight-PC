<script lang="ts">
	import Prose from '$lib/components/shared/Prose.svelte';
	import SidebarHeader from '$lib/components/shared/SidebarHeader.svelte';
	import { marked } from 'marked';
	import { User, Calendar } from '@lucide/svelte';
	import type { Post } from '$lib/features/news/schema';

	interface Props {
		post: Post;
		onclose?: () => void;
	}

	let { post, onclose }: Props = $props();
	const renderedContent = $derived(marked.parse(post.content));
</script>

<div class="flex h-full flex-col">
	<SidebarHeader {onclose} />

	<div class="grow overflow-y-auto p-6">
		<div class="mb-6 space-y-2">
			<div class="flex items-center gap-2 text-xs font-medium text-primary">
				<Calendar class="h-3.5 w-3.5" />
				{new Date(post.updated_at).toLocaleDateString(undefined, { dateStyle: 'long' })}
			</div>
			<h2 class="text-2xl leading-tight font-bold">{post.title}</h2>
			<div class="flex items-center gap-2 text-sm text-muted-foreground">
				<User class="h-4 w-4" />
				<span>Posted by {post.author}</span>
			</div>
		</div>

		<div class="prose-sm">
			<Prose content={renderedContent} />
		</div>
	</div>
</div>
