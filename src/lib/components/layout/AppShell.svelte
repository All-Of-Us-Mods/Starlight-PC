<script lang="ts">
	import { NavButton } from '$lib/components/ui/nav-button';
	import { Button } from '$lib/components/ui/button';
	import { AutoBreadcrumb } from '$lib/components/ui/breadcrumb';
	import { browser } from '$app/environment';
	import { setSidebar } from '$lib/state/sidebar.svelte';
	import { default as StarlightIcon } from '$lib/assets/starlight.svg?component';
	import { ArrowLeft, ArrowRight, Settings, Compass, House, Plus } from '@jis3r/icons';
	import { Library, Play } from '@lucide/svelte';
	import StarBackground from '$lib/components/shared/StarBackground.svelte';
	import { platform } from '@tauri-apps/plugin-os';
	import { getCurrentWindow } from '@tauri-apps/api/window';
	import { createQuery, useQueryClient } from '@tanstack/svelte-query';
	import { profileQueries } from '$lib/features/profiles/queries';
	import { launchService } from '$lib/features/profiles/launch-service';
	import type { Profile } from '$lib/features/profiles/schema';
	import { showError } from '$lib/utils/toast';
	import { gameState } from '$lib/features/profiles/game-state-service.svelte';
	import CreateProfileDialog from '$lib/features/profiles/components/CreateProfileDialog.svelte';

	type TauriWindow = Awaited<ReturnType<typeof getCurrentWindow>>;
	type Platform = 'macos' | 'windows' | 'linux' | 'other';

	let { children } = $props();

	const sidebar = setSidebar();
	const queryClient = useQueryClient();
	const activeProfileQuery = createQuery(() => profileQueries.active());

	let openCreateDialog: () => void = () => {};
	let platformName = $state<Platform>('other');
	let appWindow = $state<TauriWindow | null>(null);

	const activeProfile = $derived(activeProfileQuery.data as Profile | null);
	const sidebarWidth = $derived(sidebar.isMaximized ? '100%' : '400px');
	const canLaunch = $derived(!gameState.running && activeProfile);

	// Initialize browser-only features
	if (browser) {
		gameState.init();
		initTauri();
	}

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
			queryClient.invalidateQueries({ queryKey: ['profiles'] });
			queryClient.invalidateQueries({ queryKey: ['profiles', 'active'] });
		} catch (e) {
			showError(e);
		}
	}
</script>

<div class="app-shell" class:macos={platformName === 'macos'}>
	<!-- Star background -->
	<div class="star-container">
		<StarBackground />
	</div>

	<!-- Top Status Bar -->
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
				onclick={handleLaunchLastUsed}
				variant="outline"
				class="gap-2"
			>
				<span class="status-dot">
					{#if gameState.running}
						<span class="ping"></span>
						<span class="dot active"></span>
					{:else}
						<span class="dot"></span>
					{/if}
				</span>

				<span class="status-text">
					{#if gameState.running}
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

			{#if platformName === 'windows'}
				<div class="window-controls">
					<button aria-label="Minimize" onclick={() => appWindow?.minimize()}>
						<svg width="10" height="1"><path d="M0 0h10v1H0z" fill="currentColor" /></svg>
					</button>
					<button aria-label="Maximize" onclick={() => appWindow?.toggleMaximize()}>
						<svg width="10" height="10">
							<path d="M0 0v10h10V0H0zm9 1v8H1V1h8z" fill="currentColor" />
						</svg>
					</button>
					<button aria-label="Close" onclick={() => appWindow?.close()} class="close">
						<svg width="10" height="10">
							<path d="M0 0l10 10M10 0L0 10" stroke="currentColor" fill="none" />
						</svg>
					</button>
				</div>
			{/if}
		</div>
	</header>

	<!-- Left Navigation Bar -->
	<nav class="side-nav">
		<NavButton to="/" isPrimary={(p) => p.url.pathname === '/'} tooltip="Home">
			<House class="h-6 w-6" />
		</NavButton>
		<NavButton
			to="/explore"
			isPrimary={(p) => p.url.pathname.startsWith('/explore')}
			tooltip="Explore Mods"
		>
			<Compass class="h-6 w-6" />
		</NavButton>
		<NavButton
			to="/library"
			isPrimary={(p) => p.url.pathname.startsWith('/library')}
			tooltip="Your Library"
		>
			<Library class="h-6 w-6" />
		</NavButton>

		<div class="nav-divider"></div>

		<NavButton to={() => openCreateDialog()} tooltip="Create New">
			<Plus class="h-6 w-6" />
		</NavButton>
		<div class="hidden">
			<CreateProfileDialog onReady={(fn) => (openCreateDialog = fn)} />
		</div>

		<div class="grow"></div>

		<NavButton
			to="/settings"
			isPrimary={(p) => p.url.pathname.startsWith('/settings')}
			tooltip="Settings"
		>
			<Settings class="h-6 w-6" />
		</NavButton>
	</nav>

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

	/* Tauri drag regions */
	[data-tauri-drag-region] {
		-webkit-app-region: drag;
	}
	[data-tauri-drag-region-exclude] {
		-webkit-app-region: no-drag;
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

		&.macos .top-bar {
			padding-left: 75px;
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

	/* Top Bar */
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

	/* Window Controls (Windows) */
	.window-controls {
		@apply flex h-full;

		button {
			@apply flex h-full w-[45px] items-center justify-center transition-colors hover:bg-white/10;
			-webkit-app-region: no-drag;

			&.close:hover {
				@apply bg-red-500;
			}
		}
	}

	/* Side Navigation */
	.side-nav {
		@apply relative z-10 flex flex-col gap-2 overflow-visible bg-card/80 p-2 pt-0;
		width: var(--left-bar-width);
		grid-area: nav;
	}

	.nav-divider {
		@apply mx-auto my-2 h-px w-6 bg-accent;
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
