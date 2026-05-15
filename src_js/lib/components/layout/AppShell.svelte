<script lang="ts">
	import TopBar from './TopBar.svelte';
	import SideNav from './SideNav.svelte';
	import StarBackground from '$lib/components/shared/StarBackground.svelte';
	import type { Platform, SidebarController, WindowController } from './types';
	import type { Profile } from '$lib/features/profiles/schema';
	import type { Snippet } from 'svelte';

	let {
		children,
		platformName,
		appWindow,
		canLaunch,
		isRunning,
		activeProfile,
		sidebar,
		sidebarWidth,
		onLaunch,
		onSidebarTransitionEnd
	}: {
		children?: Snippet;
		platformName: Platform;
		appWindow: WindowController | null;
		canLaunch: boolean;
		isRunning: boolean;
		activeProfile: Profile | null;
		sidebar: SidebarController;
		sidebarWidth: string;
		onLaunch: () => void | Promise<void>;
		onSidebarTransitionEnd: (event: TransitionEvent) => void;
	} = $props();
</script>

<div class="app-shell">
	<div class="star-container">
		<StarBackground />
	</div>

	<TopBar {platformName} {appWindow} {canLaunch} {isRunning} {activeProfile} {onLaunch} />

	<SideNav />

	<main class="content-area">
		<div class="scrollbar-styled content-scroll">
			<div id="background-teleport-target" class="background-target"></div>

			<div class="content-wrapper" style:padding-right={sidebar.isOpen ? sidebarWidth : '0px'}>
				{@render children?.()}
			</div>
		</div>

		<aside
			class="app-sidebar"
			style:width={sidebar.isOpen ? sidebarWidth : '0px'}
			ontransitionend={onSidebarTransitionEnd}
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

	.app-shell {
		--left-bar-width: 4rem;
		--top-bar-height: 3rem;
	}

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
