<script lang="ts">
	import DOMPurify from 'dompurify';
	import { openUrl } from '@tauri-apps/plugin-opener';

	interface Props {
		content: string | Promise<string>;
		class?: string;
	}

	let { content, class: className = '' }: Props = $props();

	async function getSanitizedHtml(input: typeof content) {
		const raw = await input;
		return DOMPurify.sanitize(raw);
	}

	function handleLinkClick(event: MouseEvent) {
		const anchor = (event.target as HTMLElement).closest('a');
		if (anchor?.href.startsWith('http')) {
			event.preventDefault();
			openUrl(anchor.href).catch(console.error);
		}
	}
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<!-- svelte-ignore a11y_click_events_have_key_events -->
<div
	onclick={handleLinkClick}
	class="custom-prose prose prose-sm max-w-none dark:prose-invert {className}"
>
	{#await getSanitizedHtml(content)}
		<p class="text-muted-foreground">Loading content...</p>
	{:then html}
		<!-- eslint-disable-next-line svelte/no-at-html-tags -->
		{@html html}
	{:catch error}
		<p class="text-destructive">Error: {error.message}</p>
	{/await}
</div>

<style lang="postcss">
	@reference "$lib/../app.css";
	.custom-prose {
		:global(h1) {
			@apply scroll-m-20 text-4xl font-extrabold tracking-tight lg:text-5xl;
		}
		:global(h2) {
			@apply scroll-m-20 border-b pb-2 text-3xl font-semibold tracking-tight transition-colors first:mt-0;
		}
		:global(h3) {
			@apply scroll-m-20 text-2xl font-semibold tracking-tight;
		}
		:global(h4) {
			@apply scroll-m-20 text-xl font-semibold tracking-tight;
		}
		:global(p) {
			@apply leading-7 not-first:mt-6;
		}
		:global(blockquote) {
			@apply mt-6 border-l-2 pl-6 italic;
		}
		:global(code) {
			@apply relative rounded bg-muted px-[0.3rem] py-[0.2rem] font-mono text-sm font-semibold;
		}
		:global(ul) {
			@apply my-6 ml-6 list-disc [&>li]:mt-2;
		}
	}
</style>
