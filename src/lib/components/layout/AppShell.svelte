<script lang="ts">
	import { browser } from '$app/environment';
	import { setSidebar } from '$lib/state/sidebar.svelte';
	import { platform } from '@tauri-apps/plugin-os';
	import { getCurrentWindow } from '@tauri-apps/api/window';
	import { createMutation, createQuery, useQueryClient } from '@tanstack/svelte-query';
	import { profileQueries } from '$lib/features/profiles/queries';
	import { launchService } from '$lib/features/profiles/launch-service';
	import type { Profile } from '$lib/features/profiles/schema';
	import { showError } from '$lib/utils/toast';
	import { gameState } from '$lib/features/profiles/game-state.svelte';
	import { profileMutations } from '$lib/features/profiles/mutations';
	import TopBar from './TopBar.svelte';
	import SideNav from './SideNav.svelte';
	import StarBackground from '$lib/components/shared/StarBackground.svelte';
	import type { TauriWindow, Platform } from './types';

	let { children } = $props();

	const queryClient = useQueryClient();
	const sidebar = setSidebar();
	const updateLastLaunched = createMutation(() => profileMutations.updateLastLaunched(queryClient));
	const activeProfileQuery = createQuery(() => profileQueries.active());

	let platformName = $state<Platform>('other');
	let appWindow = $state<TauriWindow | null>(null);

	const activeProfile = $derived(activeProfileQuery.data as Profile | null);
	const sidebarWidth = $derived(sidebar.isMaximized ? '100%' : '400px');
	const canLaunch = $derived<boolean>(!gameState.running && !!activeProfile);

	if (browser) {
		gameState.init();
		initTauri();
	}

	$effect(() => {
		return () => {
			gameState.destroy();
		};
	});

	function initTauri() {
		const tauriWindow = window as Window & { __TAURI_INTERNALS__?: unknown };
		if (!tauriWindow.__TAURI_INTERNALS__) return;

		try {
			const os = platform();
			platformName = os === 'macos' || os === 'windows' ? os : 'linux';
			appWindow = getCurrentWindow();
		} catch (e) {
			console.error('Failed to initialize Tauri APIs:', e);
		}
	}

	function handleTransitionEnd(e: TransitionEvent) {
		if (e.propertyName === 'width' && !sidebar.isOpen) {
			sidebar.finalizeClose();
		}
	}

	async function handleLaunchLastUsed() {
		if (gameState.running) {
			showError(new Error('Among Us is already running'));
			return;
		}
		if (!activeProfile) return;

		try {
			await launchService.launchProfile(activeProfile);
			await updateLastLaunched.mutateAsync(activeProfile.id);
		} catch (e) {
			showError(e);
		}
	}
</script>

<div class="app-shell">
	<!-- Star background -->
	<div class="star-container">
		<StarBackground />
	</div>

	<!-- Top Status Bar -->
	<TopBar
		{platformName}
		{appWindow}
		{canLaunch}
		isRunning={!!gameState.running}
		{activeProfile}
		onLaunch={handleLaunchLastUsed}
	/>

	<!-- Left Navigation Bar -->
	<SideNav />

	<!-- Main Content Area -->
	<main class="content-area">
		<div class="scrollbar-styled content-scroll">
			<div id="background-teleport-target" class="background-target"></div>

			<div class="content-wrapper" style:padding-right={sidebar.isOpen ? sidebarWidth : '0px'}>
				{@render children?.()}
			</div>
		</div>

		<!-- Sidebar -->
		<aside
			class="app-sidebar"
			style:width={sidebar.isOpen ? sidebarWidth : '0px'}
			ontransitionend={handleTransitionEnd}
		>
			<div class="scrollbar-styled sidebar-scroll">
				<div class="sidebar-content" style:width={sidebarWidth} style:min-width={sidebarWidth}>
					{#if sidebar.content}
						{@render sidebar.content()}
					{/if}
				</div>
			</div>
		</aside>
	</main>
</div>

<style lang="postcss">
	@reference "$lib/../app.css";

	/* CSS Variables */
	.app-shell {
		--left-bar-width: 4rem;
		--top-bar-height: 3rem;
	}

	/* Shell Layout */
	.app-shell {
		@apply relative isolate grid h-screen overflow-hidden bg-card;
		grid-template-rows: auto 1fr;
		grid-template-columns: auto 1fr;
		grid-template-areas:
			'status status'
			'nav main';

		&::after {
			content: '';
			@apply pointer-events-none fixed z-2;
			inset: var(--top-bar-height) 0 0 var(--left-bar-width);
			border-radius: var(--radius-xl) 0 0 0;
			box-shadow:
				inset 1px 1px 15px rgba(0, 0, 0, 0.1),
				inset 1px 1px 1px rgba(255, 255, 255, 0.1);
		}
	}

	/* Star Background */
	.star-container {
		@apply pointer-events-none absolute inset-0 z-5 opacity-80;
		clip-path: polygon(
			0 0,
			100vw 0,
			100vw var(--top-bar-height),
			var(--left-bar-width) var(--top-bar-height),
			var(--left-bar-width) 100vh,
			0 100vh
		);
	}

	/* Main Content Area */
	.content-area {
		@apply absolute inset-0 z-1 overflow-hidden rounded-tl-xl bg-background;
		top: var(--top-bar-height);
		left: var(--left-bar-width);
	}

	.content-scroll {
		@apply relative h-full w-full overflow-y-auto;
	}

	.background-target {
		@apply absolute inset-0 -z-10 overflow-hidden rounded-tl-xl;
	}

	.content-wrapper {
		@apply h-full transition-[padding] duration-400 ease-in-out;
	}

	/* Sidebar */
	.app-sidebar {
		@apply absolute top-0 right-0 z-50 flex h-full flex-col items-end overflow-hidden;
		@apply border-l border-border bg-muted transition-[width] duration-400 ease-in-out;
		will-change: width;
	}

	.sidebar-scroll {
		@apply flex h-full w-full flex-col items-end overflow-y-auto;
		will-change: padding-right;
	}

	.sidebar-content {
		@apply h-full transition-[width,min-width] duration-400 ease-in-out;
	}
</style>
