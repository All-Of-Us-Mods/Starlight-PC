<script lang="ts">
	import { Skeleton } from '$lib/components/ui/skeleton';
	import PageHeader from '$lib/components/shared/PageHeader.svelte';
	import { Settings } from '@jis3r/icons';
	import { createQuery, useQueryClient } from '@tanstack/svelte-query';
	import { settingsQueries } from '$lib/features/settings/queries';
	import { settingsService } from '$lib/features/settings/settings-service';
	import type { AppSettings, GamePlatform } from '$lib/features/settings/schema';
	import { showError } from '$lib/utils/toast';
	import { invoke } from '@tauri-apps/api/core';
	import { exists } from '@tauri-apps/plugin-fs';
	import { epicService } from '$lib/features/settings/epic-service';
	import GamePathSection from '$lib/features/settings/components/GamePathSection.svelte';
	import BepInExSection from '$lib/features/settings/components/BepInExSection.svelte';
	import AppBehaviorSection from '$lib/features/settings/components/AppBehaviorSection.svelte';
	import AboutSection from '$lib/features/settings/components/AboutSection.svelte';

	const settingsQuery = createQuery(() => settingsQueries.get());
	const settings = $derived(settingsQuery.data as AppSettings | undefined);
	const queryClient = useQueryClient();

	let isLoggedIn = $state(false);
	let isCacheExists = $state(false);

	// Local form state
	let localAmongUsPath = $state('');
	let localBepInExUrl = $state('');
	let localCloseOnLaunch = $state(false);
	let localGamePlatform = $state<GamePlatform>('steam');
	let localCacheBepInEx = $state(false);

	// Auto-save state
	let initialized = $state(false);
	let pathError = $state('');

	// Per-section "Saved" indicators
	let gameConfigSaved = $state(false);
	let bepinexSaved = $state(false);
	let appBehaviorSaved = $state(false);

	// Debounce timeouts
	let pathDebounceTimeout: ReturnType<typeof setTimeout> | null = null;
	let urlDebounceTimeout: ReturnType<typeof setTimeout> | null = null;

	// Track previous values for change detection (non-reactive to avoid retriggering effects)
	let prevPath: string | null = null;
	let prevPlatform: GamePlatform | null = null;
	let prevUrl: string | null = null;
	let prevCacheBepInEx: boolean | null = null;
	let prevCloseOnLaunch: boolean | null = null;

	async function refreshEpicAuth() {
		isLoggedIn = await epicService.isLoggedIn();
	}

	async function clearXboxAppId() {
		if (settings?.xbox_app_id) {
			await settingsService.updateSettings({ xbox_app_id: undefined });
		}
	}

	async function checkCacheExists() {
		try {
			const cachePath = await settingsService.getBepInExCachePath();
			isCacheExists = await invoke<boolean>('check_bepinex_cache_exists', { cachePath });
		} catch {
			isCacheExists = false;
		}
	}

	// Validate path - returns true if valid
	async function validatePath(path: string): Promise<boolean> {
		if (!path) {
			pathError = '';
			return true; // Empty is allowed
		}
		const exePath = `${path}/Among Us.exe`;
		if (!(await exists(exePath))) {
			pathError = 'Selected folder does not contain Among Us.exe';
			return false;
		}
		pathError = '';
		return true;
	}

	// Show "Saved" indicator for a section
	function showSavedIndicator(section: 'game' | 'bepinex' | 'app') {
		if (section === 'game') {
			gameConfigSaved = true;
			setTimeout(() => (gameConfigSaved = false), 2000);
		} else if (section === 'bepinex') {
			bepinexSaved = true;
			setTimeout(() => (bepinexSaved = false), 2000);
		} else {
			appBehaviorSaved = true;
			setTimeout(() => (appBehaviorSaved = false), 2000);
		}
	}

	// Save game config (path + platform)
	async function saveGameConfig() {
		if (pathError) return;
		try {
			await settingsService.updateSettings({
				among_us_path: localAmongUsPath,
				game_platform: localGamePlatform
			});
			await settingsService.autoDetectBepInExArchitecture(localAmongUsPath);
			queryClient.invalidateQueries({ queryKey: ['settings'] });
			showSavedIndicator('game');
		} catch (e) {
			showError(e);
		}
	}

	// Save BepInEx config (url + cache toggle)
	async function saveBepInExConfig() {
		try {
			await settingsService.updateSettings({
				bepinex_url: localBepInExUrl,
				cache_bepinex: localCacheBepInEx
			});
			queryClient.invalidateQueries({ queryKey: ['settings'] });
			showSavedIndicator('bepinex');
		} catch (e) {
			showError(e);
		}
	}

	// Save app behavior (close on launch)
	async function saveAppBehavior() {
		try {
			await settingsService.updateSettings({
				close_on_launch: localCloseOnLaunch
			});
			queryClient.invalidateQueries({ queryKey: ['settings'] });
			showSavedIndicator('app');
		} catch (e) {
			showError(e);
		}
	}

	// Initialize local state from settings (only once)
	$effect(() => {
		if (settings && !initialized) {
			localAmongUsPath = settings.among_us_path ?? '';
			localBepInExUrl = settings.bepinex_url ?? '';
			localCloseOnLaunch = settings.close_on_launch ?? false;
			localGamePlatform = settings.game_platform ?? 'steam';
			localCacheBepInEx = settings.cache_bepinex ?? false;
			refreshEpicAuth();
			checkCacheExists();
			initialized = true;
		}
	});

	// Auto-save for path - debounced
	$effect(() => {
		const path = localAmongUsPath;

		if (!initialized) return;

		// First run - just record initial value
		if (prevPath === null) {
			prevPath = path;
			return;
		}

		// No change
		if (path === prevPath) return;

		prevPath = path;

		// Clear previous timeout
		if (pathDebounceTimeout) clearTimeout(pathDebounceTimeout);

		// Debounce and save
		pathDebounceTimeout = setTimeout(async () => {
			if (await validatePath(path)) {
				saveGameConfig();
			}
		}, 500);

		// Cleanup function to clear timeout when effect reruns or component unmounts
		return () => {
			if (pathDebounceTimeout) {
				clearTimeout(pathDebounceTimeout);
				pathDebounceTimeout = null;
			}
		};
	});

	// Auto-save for platform - immediate (like a toggle)
	$effect(() => {
		const platform = localGamePlatform;

		if (!initialized) return;

		if (prevPlatform === null) {
			prevPlatform = platform;
			return;
		}

		if (platform === prevPlatform) return;

		prevPlatform = platform;
		saveGameConfig();
	});

	// Auto-save for BepInEx URL - debounced
	$effect(() => {
		const url = localBepInExUrl;

		if (!initialized) return;

		if (prevUrl === null) {
			prevUrl = url;
			return;
		}

		if (url === prevUrl) return;

		prevUrl = url;

		if (urlDebounceTimeout) clearTimeout(urlDebounceTimeout);

		urlDebounceTimeout = setTimeout(() => saveBepInExConfig(), 500);

		// Cleanup function to clear timeout when effect reruns or component unmounts
		return () => {
			if (urlDebounceTimeout) {
				clearTimeout(urlDebounceTimeout);
				urlDebounceTimeout = null;
			}
		};
	});

	// Auto-save for cacheBepInEx toggle - immediate
	$effect(() => {
		const cache = localCacheBepInEx;

		if (!initialized) return;

		if (prevCacheBepInEx === null) {
			prevCacheBepInEx = cache;
			return;
		}

		if (cache === prevCacheBepInEx) return;

		prevCacheBepInEx = cache;
		saveBepInExConfig();
	});

	// Auto-save for closeOnLaunch toggle - immediate
	$effect(() => {
		const close = localCloseOnLaunch;

		if (!initialized) return;

		if (prevCloseOnLaunch === null) {
			prevCloseOnLaunch = close;
			return;
		}

		if (close === prevCloseOnLaunch) return;

		prevCloseOnLaunch = close;
		saveAppBehavior();
	});

	// Handler for path blur - validate path
	async function handlePathBlur() {
		await validatePath(localAmongUsPath);
	}
</script>

<div class="scrollbar-styled h-full overflow-y-auto px-10 py-8">
	<PageHeader
		title="Settings"
		description="Configure your Among Us path and app preferences."
		icon={Settings}
	/>

	{#if settingsQuery.isPending}
		<div class="grid max-w-4xl gap-6 lg:grid-cols-2">
			{#each [1, 2, 3] as i (i)}
				<div
					class="space-y-4 rounded-xl border border-border/50 bg-card/30 p-6 backdrop-blur-sm"
					style="animation: pulse 2s ease-in-out infinite; animation-delay: {i * 150}ms"
				>
					<Skeleton class="h-5 w-1/3" />
					<Skeleton class="h-10 w-full" />
					<Skeleton class="h-4 w-2/3" />
				</div>
			{/each}
		</div>
	{:else}
		<div class="grid max-w-4xl gap-6 lg:grid-cols-2">
			<div class="space-y-6 lg:col-span-2">
				<GamePathSection
					bind:amongUsPath={localAmongUsPath}
					bind:gamePlatform={localGamePlatform}
					bind:isLoggedIn
					{pathError}
					showSaved={gameConfigSaved}
					onRefreshAuth={refreshEpicAuth}
					onClearXboxAppId={clearXboxAppId}
					onPathBlur={handlePathBlur}
				/>
			</div>

			<BepInExSection
				bind:bepInExUrl={localBepInExUrl}
				bind:cacheBepInEx={localCacheBepInEx}
				bind:isCacheExists
				showSaved={bepinexSaved}
			/>

			<AppBehaviorSection bind:closeOnLaunch={localCloseOnLaunch} showSaved={appBehaviorSaved} />

			<div class="lg:col-span-2">
				<AboutSection />
			</div>
		</div>
	{/if}
</div>
