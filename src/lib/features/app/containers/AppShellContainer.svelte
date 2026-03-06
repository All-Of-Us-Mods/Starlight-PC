<script lang="ts">
	import { browser } from '$app/environment';
	import { setSidebar } from '$lib/features/app/state/sidebar.svelte';
	import { createMutation, createQuery, useQueryClient } from '@tanstack/svelte-query';
	import { profileQueries } from '$lib/features/profiles/queries';
	import { profilesQueryKey } from '$lib/features/profiles/profile-keys';
	import type { Profile } from '$lib/features/profiles/schema';
	import {
		gameState,
		registerProfilesInvalidateCallback
	} from '$lib/features/profiles/state/game-state.svelte';
	import { profileActions } from '$lib/features/profiles/actions';
	import { error as logError } from '@tauri-apps/plugin-log';
	import AppShell from '$lib/components/layout/AppShell.svelte';
	import type { Platform, WindowController } from '$lib/components/layout/types';
	import {
		getCurrentWindowController,
		getWindowPlatform,
		hasTauriWindowInternals
	} from '$lib/infra/tauri/window';
	import {
		canLaunchProfile,
		createShellController,
		getSidebarWidth,
		shouldFinalizeSidebarTransition
	} from '$lib/components/layout/shell-controller';

	let { children } = $props();

	const queryClient = useQueryClient();
	const sidebar = setSidebar();
	const launchProfile = createMutation(() => profileActions.launchProfile(queryClient));
	const profilesQuery = createQuery(() => profileQueries.all());
	const unregisterProfilesInvalidate = browser
		? registerProfilesInvalidateCallback(() => {
				void queryClient.invalidateQueries({ queryKey: profilesQueryKey });
			})
		: () => {};
	const shellController = createShellController({
		launchProfile: (profile: Profile) => launchProfile.mutateAsync(profile)
	});

	let platformName = $state<Platform>('other');
	let appWindow = $state<WindowController | null>(null);

	const activeProfile = $derived.by(() => {
		const profiles = (profilesQuery.data as Profile[] | undefined) ?? [];
		return (
			profiles
				.filter((profile) => profile.last_launched_at != null)
				.toSorted((a, b) => (b.last_launched_at ?? 0) - (a.last_launched_at ?? 0))[0] ?? null
		);
	});
	const sidebarWidth = $derived(getSidebarWidth(sidebar.isMaximized));
	const canLaunch = $derived<boolean>(canLaunchProfile(activeProfile));

	if (browser) {
		gameState.init();
		initTauri();
	}

	$effect(() => {
		return () => {
			unregisterProfilesInvalidate();
			gameState.destroy();
		};
	});

	function initTauri() {
		if (!hasTauriWindowInternals()) return;

		try {
			platformName = getWindowPlatform();
			appWindow = getCurrentWindowController();
		} catch (e) {
			void logError(`Failed to initialize Tauri APIs: ${e}`);
		}
	}

	function handleTransitionEnd(e: TransitionEvent) {
		if (shouldFinalizeSidebarTransition(e, sidebar.isOpen)) {
			sidebar.finalizeClose();
		}
	}

	async function handleLaunchLastUsed() {
		await shellController.launchActiveProfile(activeProfile);
	}
</script>

<AppShell
	{children}
	{platformName}
	{appWindow}
	{sidebar}
	{sidebarWidth}
	{canLaunch}
	isRunning={!!gameState.running}
	{activeProfile}
	onLaunch={handleLaunchLastUsed}
	onSidebarTransitionEnd={handleTransitionEnd}
/>
