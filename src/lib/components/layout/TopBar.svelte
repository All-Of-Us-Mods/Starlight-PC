<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { AutoBreadcrumb } from '$lib/components/ui/breadcrumb';
	import { default as StarlightIcon } from '$lib/assets/starlight.svg?component';
	import { ArrowLeft, ArrowRight, House } from '@jis3r/icons';
	import { Play } from '@lucide/svelte';
	import WindowControls from './WindowControls.svelte';
	import type { TauriWindow, Platform } from './types';
	import type { Profile } from '$lib/features/profiles/schema';

	let {
		platformName,
		appWindow,
		canLaunch,
		isRunning,
		activeProfile,
		onLaunch
	}: {
		platformName: Platform;
		appWindow: TauriWindow | null;
		canLaunch: boolean;
		isRunning: boolean;
		activeProfile: Profile | null;
		onLaunch: () => void;
	} = $props();
</script>

<header data-tauri-drag-region class="top-bar">
	<div data-tauri-drag-region class="logo">
		<StarlightIcon class="h-6 w-6" />
	</div>

	<nav data-tauri-drag-region class="nav-controls">
		<div data-tauri-drag-region-exclude class="nav-arrows">
			<Button variant="navigation" aria-label="Go back" onclick={() => history.back()}>
				<ArrowLeft />
			</Button>
			<Button variant="navigation" aria-label="Go forward" onclick={() => history.forward()}>
				<ArrowRight />
			</Button>
		</div>
		<AutoBreadcrumb homeIcon={House} maxItems={4} />
	</nav>

	<div data-tauri-drag-region class="top-bar-actions">
		<Button
			data-tauri-drag-region-exclude
			disabled={!canLaunch}
			onclick={onLaunch}
			variant="outline"
			class="gap-2"
		>
			<span class="status-dot">
				{#if isRunning}
					<span class="ping"></span>
					<span class="dot active"></span>
				{:else}
					<span class="dot"></span>
				{/if}
			</span>

			<span class="status-text">
				{#if isRunning}
					Running
				{:else if activeProfile}
					Launch {activeProfile.name}
				{:else}
					No instances running
				{/if}
			</span>

			{#if canLaunch}
				<Play class="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
			{/if}
		</Button>

		<WindowControls {appWindow} {platformName} />
	</div>
</header>

<style lang="postcss">
	@reference "$lib/../app.css";

	/* Tauri drag regions */
	[data-tauri-drag-region] {
		-webkit-app-region: drag;
	}
	[data-tauri-drag-region-exclude] {
		-webkit-app-region: no-drag;
	}

	.top-bar {
		@apply relative z-10 flex items-center bg-card/80;
		height: var(--top-bar-height);
		grid-area: status;
	}

	.logo {
		@apply flex items-center gap-2 p-5;
	}

	.nav-controls {
		@apply flex items-center gap-1;
	}

	.nav-arrows {
		@apply mr-4 flex items-center gap-1 rounded-full bg-border;
	}

	.top-bar-actions {
		@apply ml-auto flex h-full items-center gap-2;
	}

	/* Status Button Elements */
	.status-dot {
		@apply relative flex h-2 w-2 shrink-0;
	}

	.ping {
		@apply absolute inline-flex h-full w-full animate-ping rounded-full bg-green-500 opacity-75;
	}

	.dot {
		@apply relative inline-flex h-2 w-2 rounded-full bg-muted-foreground/50;

		&.active {
			@apply bg-green-500;
		}
	}

	.status-text {
		@apply truncate text-muted-foreground;
	}
</style>
