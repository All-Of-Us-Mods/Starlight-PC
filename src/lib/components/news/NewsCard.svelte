<script lang="ts">
	import * as Card from '$lib/components/ui/card';
	import Prose from '$lib/components/shared/Prose.svelte';
	import { marked } from 'marked';
	import { ArrowRight } from '@jis3r/icons';
	import { User, Calendar } from '@lucide/svelte';
	import type { Post } from '$lib/features/news/schema';

	interface Props {
		post: Post;
		isSelected?: boolean;
		onclick?: () => void;
	}

	let { post, isSelected = false, onclick }: Props = $props();

	function safeParseMarkdown(content: string): string {
		try {
			return marked.parse(content, { async: false });
		} catch {
			return '<p>Failed to render content</p>';
		}
	}

	const renderedContent = $derived(safeParseMarkdown(post.content));
</script>

<Card.Root
	{onclick}
	class="group flex h-72 cursor-pointer flex-col transition-all hover:shadow-lg {isSelected
		? 'border-primary ring-1 ring-primary'
		: 'hover:border-primary/50'}"
>
	<Card.Header class="pb-3">
		<div class="mb-2 flex items-center justify-between text-xs text-muted-foreground">
			<div class="flex items-center gap-1.5">
				<User class="h-3.5 w-3.5" />
				<span class="font-medium text-foreground/80">{post.author}</span>
			</div>
			<div class="flex items-center gap-1.5">
				<Calendar class="h-3.5 w-3.5" />
				{new Date(post.updated_at).toLocaleDateString()}
			</div>
		</div>

		<Card.Title class="text-lg leading-snug transition-colors group-hover:text-primary">
			{post.title}
		</Card.Title>
	</Card.Header>

	<Card.Content class="relative line-clamp-4 flex-1 overflow-hidden px-6 py-0 text-sm">
		<div class="pointer-events-none prose-sm text-muted-foreground">
			<Prose content={renderedContent} />
		</div>
		<div
			class="pointer-events-none absolute right-0 bottom-0 left-0 h-16 bg-linear-to-t from-card to-transparent"
		></div>
	</Card.Content>

	<Card.Footer class="pt-2">
		<div
			class="group/btn flex items-center gap-1 text-sm font-semibold text-primary hover:underline"
		>
			Read full post
			<ArrowRight />
		</div>
	</Card.Footer>
</Card.Root>
