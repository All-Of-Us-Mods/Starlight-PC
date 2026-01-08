<script lang="ts">
	import { openUrl } from '@tauri-apps/plugin-opener';
	import { Github, Heart, ExternalLink } from '@lucide/svelte';
	import { Button } from '$lib/components/ui/button';
	import { getVersion } from '@tauri-apps/api/app';

	const APP_NAME = 'Starlight PC';
	const GITHUB_URL = 'https://github.com/All-Of-Us-Mods/Starlight-PC';

	function handleGitHubClick() {
		openUrl(GITHUB_URL);
	}

	// Generate deterministic star positions
	const stars = Array.from({ length: 24 }, (_, i) => ({
		left: `${(i * 17 + 7) % 100}%`,
		top: `${(i * 23 + 11) % 100}%`,
		size: (i % 3) + 1,
		delay: `${(i * 0.15) % 2}s`,
		duration: `${2 + (i % 3)}s`
	}));
</script>

<div class="relative overflow-hidden rounded-xl border border-border/50 p-8">
	<!-- Subtle star background -->
	<div class="pointer-events-none absolute inset-0 overflow-hidden">
		{#each stars as star, i (i)}
			<div
				class="star absolute rounded-full bg-primary/30"
				style="
					left: {star.left};
					top: {star.top};
					width: {star.size}px;
					height: {star.size}px;
					animation: twinkle {star.duration} ease-in-out infinite;
					animation-delay: {star.delay};
				"
			></div>
		{/each}
	</div>

	<div class="relative z-10 flex flex-col items-center text-center sm:flex-row sm:text-left">
		<!-- Info content -->
		<div class="flex-1">
			<div class="mb-3 flex flex-col items-center gap-2 sm:flex-row sm:items-baseline">
				<h2 class="text-xl font-semibold">{APP_NAME}</h2>
				<span class="rounded-full bg-muted px-2.5 py-0.5 text-xs font-medium text-muted-foreground">
					{#await getVersion()}
						v...
					{:then version}
						v{version}
					{/await}
				</span>
			</div>

			<div
				class="mb-5 flex flex-wrap items-center justify-center gap-x-4 gap-y-1 text-xs text-muted-foreground/70 sm:justify-start"
			>
				<span class="flex items-center gap-1">
					<Heart class="h-3 w-3" />
					{new Date().getFullYear()} All Of Us Mods
				</span>
				<span class="hidden sm:inline">|</span>
				<span>GNU GPLv3 License</span>
			</div>

			<div class="flex flex-wrap justify-center gap-3 sm:justify-start">
				<Button variant="outline" size="sm" onclick={handleGitHubClick} class="gap-2">
					<Github class="h-4 w-4" />
					View Source
					<ExternalLink class="h-3 w-3 text-muted-foreground" />
				</Button>
			</div>
		</div>
	</div>
</div>

<style>
	@keyframes twinkle {
		0%,
		100% {
			opacity: 0.15;
			transform: scale(1);
		}
		50% {
			opacity: 0.5;
			transform: scale(1);
		}
	}

	.star {
		will-change: opacity;
	}
</style>
