<script lang="ts">
	import { NavButton } from '$lib/components/ui/nav-button';
	import { Button } from '$lib/components/ui/button';
	import { AutoBreadcrumb } from '$lib/components/ui/breadcrumb';
	import { browser } from '$app/environment';
	import { setSidebar } from '$lib/state/sidebar.svelte';
	import { default as StarlightIcon } from '$lib/assets/starlight.svg?component';
	import { ArrowLeft, ArrowRight, Settings, Compass, House, Plus, Maximize } from '@jis3r/icons';
	import { Library, Play, X } from '@lucide/svelte';
	import StarBackground from '$lib/components/shared/StarBackground.svelte';
	import { platform } from '@tauri-apps/plugin-os';
	import { getCurrentWindow } from '@tauri-apps/api/window';
	import { createQuery, useQueryClient } from '@tanstack/svelte-query';
	import { profileQueries } from '$lib/features/profiles/queries';
	import { launchService } from '$lib/features/profiles/launch-service';
	import type { Profile } from '$lib/features/profiles/schema';
	import { showToastError } from '$lib/utils/toast';
	import { gameState } from '$lib/features/profiles/game-state-service.svelte';
	import CreateProfileDialog from '$lib/features/profiles/components/CreateProfileDialog.svelte';

	if (browser) {
		gameState.init();
	}

	let { children } = $props();
	const sidebar = setSidebar();
	let dialogRef = $state({ open: () => {} });

	type TauriWindow = Awaited<ReturnType<typeof getCurrentWindow>>;

	// Detect platform for layout adjustments
	let platformName = $state<'macos' | 'windows' | 'linux' | 'other'>('other');
	let appWindow = $state<TauriWindow | null>(null);

	if (browser && (window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__) {
		try {
			const osType = platform();
			if (osType === 'macos') platformName = 'macos';
			else if (osType === 'windows') platformName = 'windows';
			else platformName = 'linux';

			appWindow = getCurrentWindow();
		} catch (e) {
			console.error('Failed to initialize Tauri APIs:', e);
		}
	}
	const minimize = () => appWindow?.minimize();
	const toggleMaximize = () => appWindow?.toggleMaximize();
	const close = () => appWindow?.close();

	function handleTransitionEnd(e: TransitionEvent) {
		if (e.propertyName === 'grid-template-columns' && !sidebar.isOpen) {
			sidebar.finalizeClose();
		}
	}

	const queryClient = useQueryClient();
	const activeProfileQuery = createQuery(() => profileQueries.active());
	const activeProfile = $derived((activeProfileQuery.data ?? null) as Profile | null);

	async function handleLaunchLastUsed() {
		if (gameState.running) {
			showToastError(new Error('Among Us is already running'));
			return;
		}
		if (!activeProfile) return;
		try {
			await launchService.launchProfile(activeProfile);
			queryClient.invalidateQueries({ queryKey: ['profiles'] });
			queryClient.invalidateQueries({ queryKey: ['profiles', 'active'] });
		} catch (e) {
			showToastError(e);
		}
	}

	let isSidebarMaximized = $state(false);

	function toggleSidebarMaximize() {
		isSidebarMaximized = !isSidebarMaximized;
	}
</script>

<div
	class="app-shell relative isolate grid h-screen auto-rows-[auto_1fr] grid-cols-[auto_1fr] overflow-hidden bg-card
		[--left-bar-width:4rem] [--top-bar-height:3rem]
		[grid-template-areas:'status_status'_'nav_dummy']"
	style="--right-bar-width: {isSidebarMaximized ? '100%' : '400px'}"
>
	<!-- Star background -->
	<div class="star-container pointer-events-none absolute inset-0 z-5 opacity-80">
		<StarBackground />
	</div>

	<!-- Top Status Bar -->
	<div
		data-tauri-drag-region
		class="relative z-10 flex h-(--top-bar-height) items-center bg-card/80 [grid-area:status]
			{platformName === 'macos' ? 'pl-[75px]' : ''}"
	>
		<!-- Left Side: Logo/Brand -->
		<div data-tauri-drag-region class="flex items-center gap-2 p-5">
			<StarlightIcon class="h-6 w-6" />
		</div>

		<!-- Center-Left: Navigation Controls -->
		<div data-tauri-drag-region class="flex items-center gap-1">
			<div
				data-tauri-drag-region-exclude
				class="mr-4 flex items-center gap-1 rounded-full bg-border"
			>
				<Button variant="navigation" aria-label="Go back" onclick={() => history.back()}>
					<ArrowLeft />
				</Button>
				<Button variant="navigation" aria-label="Go forward" onclick={() => history.forward()}>
					<ArrowRight />
				</Button>
			</div>
			<AutoBreadcrumb homeIcon={House} maxItems={4} />
		</div>

		<!-- Right Side: Spacer for Windows controls or additional tools -->
		<div data-tauri-drag-region class="ml-auto flex h-full items-center gap-2">
			<Button
				data-tauri-drag-region-exclude
				disabled={gameState.running || !activeProfile}
				onclick={handleLaunchLastUsed}
				variant="outline"
				class="gap-2"
			>
				<!-- Status Indicator Dot -->
				<span class="relative flex h-2 w-2 shrink-0">
					{#if gameState.running}
						<span
							class="absolute inline-flex h-full w-full animate-ping rounded-full bg-green-500 opacity-75"
						></span>
						<span class="relative inline-flex h-2 w-2 rounded-full bg-green-500"></span>
					{:else}
						<span class="relative inline-flex h-2 w-2 rounded-full bg-muted-foreground/50"></span>
					{/if}
				</span>

				<!-- Status Text -->
				<span class="truncate text-muted-foreground">
					{#if gameState.running}
						Running
					{:else if activeProfile}
						Launch {activeProfile.name}
					{:else}
						No instances running
					{/if}
				</span>

				<!-- Launch Icon -->
				{#if !gameState.running && activeProfile}
					<Play class="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
				{/if}
			</Button>
			{#if platformName === 'windows'}
				<div class="flex h-full">
					<button aria-label="Minimize" onclick={minimize} class="window-control">
						<svg width="10" height="1" viewBox="0 0 10 1"
							><path d="M0 0h10v1H0z" fill="currentColor" /></svg
						>
					</button>
					<button aria-label="Maximize" onclick={toggleMaximize} class="window-control">
						<svg width="10" height="10" viewBox="0 0 10 10"
							><path d="M0 0v10h10V0H0zm9 1v8H1V1h8z" fill="currentColor" /></svg
						>
					</button>
					<button aria-label="Close" onclick={close} class="window-control hover:bg-red-500!">
						<svg width="10" height="10" viewBox="0 0 10 10"
							><path d="M0 0l10 10M10 0L0 10" stroke="currentColor" fill="none" /></svg
						>
					</button>
				</div>
			{/if}
		</div>
	</div>

	<!-- Left Navigation Bar -->
	<nav
		class="relative z-10 flex w-(--left-bar-width) flex-col gap-2 overflow-visible bg-card/80 p-2 pt-0 [grid-area:nav]"
	>
		<NavButton to="/" isPrimary={(page) => page.url.pathname === '/'} tooltip="Home">
			<House class="h-6 w-6" />
		</NavButton>
		<NavButton
			to="/explore"
			isPrimary={(page) => page.url.pathname.startsWith('/explore')}
			tooltip="Explore Mods"
		>
			<Compass class="h-6 w-6" />
		</NavButton>
		<NavButton
			to="/library"
			isPrimary={(page) => page.url.pathname.startsWith('/library')}
			tooltip="Your Library"
		>
			<Library class="h-6 w-6" />
		</NavButton>
		<div class="mx-auto my-2 h-px w-6 bg-accent"></div>
		<NavButton to={() => dialogRef.open()} tooltip="Create New">
			<Plus class="h-6 w-6" />
		</NavButton>
		<div class="hidden">
			<CreateProfileDialog {dialogRef} />
		</div>
		<div class="flex grow"></div>
		<NavButton
			to="/settings"
			isPrimary={(page) => page.url.pathname.startsWith('/settings')}
			tooltip="Settings"
		>
			<Settings class="h-6 w-6" />
		</NavButton>
	</nav>

	<!-- Main Content Area -->
	<div
	class="absolute inset-0 top-(--top-bar-height) left-(--left-bar-width) z-1 grid h-[calc(100vh-var(--top-bar-height))] overflow-hidden rounded-tl-xl bg-background transition-[grid-template-columns] duration-400 ease-in-out"
	style="grid-template-columns: 1fr {sidebar.isOpen ? 'var(--right-bar-width)' : '0px'}"
	ontransitionend={handleTransitionEnd}
	>
		<div class="scrollbar-styled relative h-full grow overflow-y-auto">
			<div
				id="background-teleport-target"
				class="absolute -z-10 h-full w-[calc(100%-var(--right-bar-width))] overflow-hidden rounded-tl-xl"
			></div>
			{@render children?.()}
		</div>

		<!-- Right Sidebar -->
		<div
			class="app-sidebar relative mt-px flex shrink-0 flex-col overflow-hidden border-l border-border bg-muted"
			style="width: var(--right-bar-width);"
		>
			<div class="scrollbar-styled relative grow overflow-y-auto">
				{#if sidebar.content}
					<Button variant="ghost" size="icon" onclick={toggleSidebarMaximize}>
						<Maximize class="h-4 w-4" />
					</Button>

					{@render sidebar.content()}
				{/if}
			</div>
		</div>
	</div>
</div>

<style lang="postcss">
	@reference "$lib/../app.css";

	[data-tauri-drag-region] {
		-webkit-app-region: drag;
	}
	[data-tauri-drag-region-exclude] {
		-webkit-app-region: no-drag;
	}

	.star-container {
		clip-path: polygon(
			0 0,
			100vw 0,
			100vw var(--top-bar-height),
			var(--left-bar-width) var(--top-bar-height),
			var(--left-bar-width) 100vh,
			0 100vh
		);
	}

	.app-shell::after {
		content: '';
		position: fixed;
		z-index: 2;
		pointer-events: none;
		inset: var(--top-bar-height) 0 0 var(--left-bar-width);
		border-radius: var(--radius-xl) 0 0 0;
		box-shadow:
			inset 1px 1px 15px rgba(0, 0, 0, 0.1),
			inset 1px 1px 1px rgba(255, 255, 255, 0.1);
	}

	.window-control {
		@apply flex h-full w-[45px] items-center justify-center transition-colors hover:bg-white/10;
		-webkit-app-region: no-drag;
	}
</style>
