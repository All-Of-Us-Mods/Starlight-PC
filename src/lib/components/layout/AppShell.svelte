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
	import { createQuery } from '@tanstack/svelte-query';
	import { profileQueries } from '$lib/features/profiles/queries';
	import { launchService } from '$lib/features/profiles/launch-service';
	import type { Profile } from '$lib/features/profiles/schema';
	import { showToastError } from '$lib/utils/toast';
	import CreateProfileDialog from '$lib/features/profiles/components/CreateProfileDialog.svelte';

	let { children } = $props();
	const sidebar = setSidebar();
	let dialogRef = $state({ open: () => {} });

	// Detect platform for layout adjustments
	let platformName = $state<'macos' | 'windows' | 'linux' | 'other'>('other');
	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	let appWindow = $state<any>(null);

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

	const activeProfileQuery = createQuery(() => profileQueries.active());
	const activeProfile = $derived((activeProfileQuery.data ?? null) as Profile | null);

	async function handleLaunchLastUsed() {
		if (!activeProfile) return;
		try {
			await launchService.launchProfile(activeProfile);
		} catch (e) {
			showToastError(e);
		}
	}
</script>

<div
	class="app-shell relative isolate grid h-screen auto-rows-[auto_1fr] grid-cols-[auto_1fr] overflow-hidden bg-card
		[--left-bar-width:4rem] [--right-bar-width:300px] [--top-bar-height:3rem]
		[grid-template-areas:'status_status'_'nav_dummy']"
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
			{#if activeProfile}
				<Button
					data-tauri-drag-region-exclude
					size="sm"
					variant="ghost"
					onclick={handleLaunchLastUsed}
					class="gap-2"
				>
					<Play class="h-4 w-4 fill-current" />
					<span class="hidden sm:inline">Launch {activeProfile.name}</span>
				</Button>
			{/if}
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
		class="absolute inset-0 top-(--top-bar-height) left-(--left-bar-width) z-1 grid h-[calc(100vh-var(--top-bar-height))] overflow-hidden rounded-tl-xl bg-background
			transition-[grid-template-columns] duration-400 ease-in-out
			{sidebar.isOpen ? 'grid-cols-[1fr_var(--right-bar-width)]' : 'grid-cols-[1fr_0px]'}"
		ontransitionend={handleTransitionEnd}
	>
		<div class="relative h-full grow overflow-hidden">
			<div
				id="background-teleport-target"
				class="absolute -z-10 h-full w-[calc(100%-var(--right-bar-width))] overflow-hidden rounded-tl-xl"
			></div>
			{@render children?.()}
		</div>

		<!-- Right Sidebar -->
		<div
			class="app-sidebar relative mt-px flex shrink-0 flex-col overflow-visible border-l border-border bg-muted"
			style="width: var(--right-bar-width);"
		>
			<div class="relative grow overflow-y-auto pb-12">
				{#if sidebar.content}
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
