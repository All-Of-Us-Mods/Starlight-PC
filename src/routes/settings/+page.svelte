<script lang="ts">
	import { Skeleton } from '$lib/components/ui/skeleton';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { Label } from '$lib/components/ui/label';
	import { Switch } from '$lib/components/ui/switch';
	import PageHeader from '$lib/components/shared/PageHeader.svelte';
	import { Settings, RefreshCw, Download, Trash2, Check } from '@jis3r/icons';
	import { Github, Heart, ExternalLink } from '@lucide/svelte';
	import { createQuery, createMutation, useQueryClient } from '@tanstack/svelte-query';
	import { settingsQueries } from '$lib/features/settings/queries';
	import { settingsMutations } from '$lib/features/settings/mutations';
	import { settingsService } from '$lib/features/settings/settings-service';
	import type { AppSettings, GamePlatform } from '$lib/features/settings/schema';
	import { showError, showSuccess } from '$lib/utils/toast';
	import { invoke } from '@tauri-apps/api/core';
	import { exists } from '@tauri-apps/plugin-fs';
	import { listen, type UnlistenFn } from '@tauri-apps/api/event';
	import { open as openDialog } from '@tauri-apps/plugin-dialog';
	import { openUrl } from '@tauri-apps/plugin-opener';
	import { getVersion } from '@tauri-apps/api/app';
	import { error as logError } from '@tauri-apps/plugin-log';
	import { epicService } from '$lib/features/settings/epic-service';
	import EpicLoginDialog from '$lib/features/settings/EpicLoginDialog.svelte';
	import type { BepInExProgress } from '$lib/features/profiles/schema';

	const APP_NAME = 'Starlight PC';
	const GITHUB_URL = 'https://github.com/All-Of-Us-Mods/Starlight-PC';

	const settingsQuery = createQuery(() => settingsQueries.get());
	const settings = $derived(settingsQuery.data as AppSettings | undefined);
	const queryClient = useQueryClient();

	// ============ MUTATIONS ============

	const updateSettingsMutation = createMutation(() => settingsMutations.update(queryClient));

	// ============ SHARED STATE ============

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

	// Saved indicator timeouts
	let gameConfigSavedTimeout: ReturnType<typeof setTimeout> | null = null;
	let bepinexSavedTimeout: ReturnType<typeof setTimeout> | null = null;
	let appBehaviorSavedTimeout: ReturnType<typeof setTimeout> | null = null;

	// Track previous values for change detection (non-reactive to avoid retriggering effects)
	let prevPath: string | null = null;
	let prevPlatform: GamePlatform | null = null;
	let prevUrl: string | null = null;
	let prevCacheBepInEx: boolean | null = null;
	let prevCloseOnLaunch: boolean | null = null;

	// ============ GAME PATH SECTION STATE ============

	let isDetecting = $state(false);
	let epicLoginOpen = $state(false);

	// Track previous values to detect user-initiated changes for clearing xbox_app_id
	let gamePathPrevPath = $state<string | null>(null);
	let gamePathPrevPlatform = $state<GamePlatform | null>(null);

	// ============ BEPINEX SECTION STATE ============

	let isCacheDownloading = $state(false);
	let cacheDownloadProgress = $state(0);

	// ============ ABOUT SECTION ============

	// Generate deterministic star positions
	const stars = Array.from({ length: 24 }, (_, i) => ({
		left: `${(i * 17 + 7) % 100}%`,
		top: `${(i * 23 + 11) % 100}%`,
		size: (i % 3) + 1,
		delay: `${(i * 0.15) % 2}s`,
		duration: `${2 + (i % 3)}s`
	}));

	// ============ SHARED FUNCTIONS ============

	async function refreshEpicAuth() {
		isLoggedIn = await epicService.isLoggedIn();
	}

	async function clearXboxAppId() {
		if (settings?.xbox_app_id) {
			await updateSettingsMutation.mutateAsync({ xbox_app_id: undefined });
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
			if (gameConfigSavedTimeout) clearTimeout(gameConfigSavedTimeout);
			gameConfigSaved = true;
			gameConfigSavedTimeout = setTimeout(() => (gameConfigSaved = false), 2000);
		} else if (section === 'bepinex') {
			if (bepinexSavedTimeout) clearTimeout(bepinexSavedTimeout);
			bepinexSaved = true;
			bepinexSavedTimeout = setTimeout(() => (bepinexSaved = false), 2000);
		} else {
			if (appBehaviorSavedTimeout) clearTimeout(appBehaviorSavedTimeout);
			appBehaviorSaved = true;
			appBehaviorSavedTimeout = setTimeout(() => (appBehaviorSaved = false), 2000);
		}
	}

	// Save game config (path + platform)
	async function saveGameConfig() {
		if (pathError) return;
		try {
			await updateSettingsMutation.mutateAsync({
				among_us_path: localAmongUsPath,
				game_platform: localGamePlatform
			});
			const newUrl = await settingsService.autoDetectBepInExArchitecture(localAmongUsPath);
			if (newUrl) {
				// Sync local state to prevent desync with backend
				localBepInExUrl = newUrl;
				prevUrl = newUrl;
			}
			showSavedIndicator('game');
		} catch (e) {
			showError(e);
		}
	}

	// Save BepInEx config (url + cache toggle)
	async function saveBepInExConfig() {
		try {
			await updateSettingsMutation.mutateAsync({
				bepinex_url: localBepInExUrl,
				cache_bepinex: localCacheBepInEx
			});
			showSavedIndicator('bepinex');
		} catch (e) {
			showError(e);
		}
	}

	// Save app behavior (close on launch)
	async function saveAppBehavior() {
		try {
			await updateSettingsMutation.mutateAsync({
				close_on_launch: localCloseOnLaunch
			});
			showSavedIndicator('app');
		} catch (e) {
			showError(e);
		}
	}

	// ============ GAME PATH SECTION FUNCTIONS ============

	// Clear xbox_app_id when path or platform changes (skip initial mount)
	$effect(() => {
		if (gamePathPrevPath === null || gamePathPrevPlatform === null) {
			// First run - just record initial values without triggering clear
			gamePathPrevPath = localAmongUsPath;
			gamePathPrevPlatform = localGamePlatform;
			return;
		}
		if (localAmongUsPath !== gamePathPrevPath || localGamePlatform !== gamePathPrevPlatform) {
			gamePathPrevPath = localAmongUsPath;
			gamePathPrevPlatform = localGamePlatform;
			clearXboxAppId();
		}
	});

	async function handleAutoDetect() {
		isDetecting = true;
		try {
			const path = await invoke<string | null>('detect_among_us');
			if (path) {
				localAmongUsPath = path;
				try {
					const platform = await invoke<string>('get_game_platform', { path });
					localGamePlatform = platform as GamePlatform;
				} catch (platformError) {
					logError(`Platform detection failed: ${platformError}`);
				}
				showSuccess('Among Us path detected successfully');
			} else {
				showError('Could not auto-detect Among Us installation');
			}
		} catch (e) {
			showError(e);
		} finally {
			isDetecting = false;
		}
	}

	async function handleBrowse() {
		try {
			const selected = await openDialog({
				directory: true,
				multiple: false,
				title: 'Select Among Us Installation Folder'
			});
			if (selected) {
				localAmongUsPath = selected;
				try {
					const platform = await invoke<string>('get_game_platform', { path: selected });
					localGamePlatform = platform as GamePlatform;
				} catch (platformError) {
					logError(`Platform detection failed: ${platformError}`);
				}
			}
		} catch (e) {
			showError(e);
		}
	}

	// Handler for path blur - validate path
	async function handlePathBlur() {
		await validatePath(localAmongUsPath);
	}

	// ============ BEPINEX SECTION FUNCTIONS ============

	async function handleDownloadToCache() {
		if (!localBepInExUrl) {
			showError('BepInEx URL is required');
			return;
		}

		isCacheDownloading = true;
		cacheDownloadProgress = 0;
		let unlisten: UnlistenFn | undefined;

		try {
			unlisten = await listen<BepInExProgress>('bepinex-progress', (event) => {
				cacheDownloadProgress = event.payload.progress;
			});

			const cachePath = await settingsService.getBepInExCachePath();
			await invoke('download_bepinex_to_cache', {
				url: localBepInExUrl,
				cachePath
			});
			isCacheExists = true;
			showSuccess('BepInEx downloaded to cache');
		} catch (e) {
			showError(e);
		} finally {
			unlisten?.();
			isCacheDownloading = false;
			cacheDownloadProgress = 0;
		}
	}

	async function handleClearCache() {
		try {
			const cachePath = await settingsService.getBepInExCachePath();
			await invoke('clear_bepinex_cache', { cachePath });
			isCacheExists = false;
			showSuccess('Cache cleared');
		} catch (e) {
			showError(e);
		}
	}

	// ============ ABOUT SECTION FUNCTIONS ============

	function handleGitHubClick() {
		openUrl(GITHUB_URL);
	}

	// ============ AUTO-SAVE EFFECTS ============

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
			const isValid = await validatePath(path);
			// Only save if path hasn't changed during validation
			if (localAmongUsPath === path && isValid) {
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
			<!-- Game Configuration Section -->
			<div class="space-y-6 lg:col-span-2">
				<div class="rounded-xl border border-border/50 bg-card/30 p-6 backdrop-blur-sm">
					<div class="mb-4 flex items-center justify-between">
						<h2 class="text-lg font-semibold tracking-tight">Game Configuration</h2>
						{#if gameConfigSaved}
							<span
								class="flex animate-in items-center gap-1.5 text-xs font-medium text-green-500 duration-200 fade-in"
							>
								<Check size={14} />
								Saved
							</span>
						{/if}
					</div>
					<div class="space-y-4">
						<div class="space-y-2">
							<Label for="among-us-path">Among Us Installation Path</Label>
							<div class="flex gap-2">
								<Input
									id="among-us-path"
									bind:value={localAmongUsPath}
									placeholder="C:\Program Files (x86)\Steam\steamapps\common\Among Us"
									onblur={handlePathBlur}
									class={pathError ? 'border-destructive focus-visible:ring-destructive' : ''}
								/>
								<Button variant="outline" onclick={handleBrowse}>Browse</Button>
								<Button variant="outline" onclick={handleAutoDetect} disabled={isDetecting}>
									{#if isDetecting}
										<RefreshCw class="h-4 w-4 animate-spin" />
									{:else}
										<RefreshCw class="h-4 w-4" />
									{/if}
								</Button>
							</div>
							<p class="text-sm text-muted-foreground">
								The folder where Among Us is installed (contains "Among Us.exe")
							</p>
							{#if pathError}
								<p class="text-sm text-destructive">{pathError}</p>
							{/if}
						</div>

						<div class="space-y-2">
							<Label>Game Platform</Label>
							<div class="flex gap-2">
								<Button
									variant={localGamePlatform === 'steam' ? 'default' : 'outline'}
									onclick={() => (localGamePlatform = 'steam')}
									class="flex-1"
								>
									Steam / Itch.io
								</Button>
								<Button
									variant={localGamePlatform === 'epic' ? 'default' : 'outline'}
									onclick={() => (localGamePlatform = 'epic')}
									class="flex-1"
								>
									Epic Games
								</Button>
								<Button
									variant={localGamePlatform === 'xbox' ? 'default' : 'outline'}
									onclick={() => (localGamePlatform = 'xbox')}
									class="flex-1"
								>
									Xbox / MS Store
								</Button>
							</div>
							<p class="text-sm text-muted-foreground">
								{#if localGamePlatform === 'steam'}
									Steam installation
								{:else if localGamePlatform === 'epic'}
									Epic Games installation (requires Epic Games login)
								{:else}
									Xbox / Microsoft Store installation
								{/if}
							</p>
						</div>
						{#if localGamePlatform === 'epic'}
							<div class="flex items-center justify-between rounded-md bg-muted/50 p-3">
								<div class="space-y-0.5">
									<p class="text-sm font-medium">Account Status</p>
									<p class="text-sm text-muted-foreground">
										{#if isLoggedIn}
											<span class="text-green-500">Logged in</span>
										{:else}
											<span class="text-orange-500">Not logged in</span>
										{/if}
									</p>
								</div>
								<Button variant="outline" size="sm" onclick={() => (epicLoginOpen = true)}>
									{#if isLoggedIn}
										Manage Account
									{:else}
										Login to Epic Games
									{/if}
								</Button>
							</div>
						{/if}
					</div>
				</div>
			</div>

			<!-- BepInEx Configuration Section -->
			<div class="rounded-xl border border-border/50 bg-card/30 p-6 backdrop-blur-sm">
				<div class="mb-4 flex items-center justify-between">
					<h2 class="text-lg font-semibold tracking-tight">BepInEx Configuration</h2>
					{#if bepinexSaved}
						<span
							class="flex animate-in items-center gap-1.5 text-xs font-medium text-green-500 duration-200 fade-in"
						>
							<Check size={14} />
							Saved
						</span>
					{/if}
				</div>
				<div class="space-y-4">
					<div class="space-y-2">
						<Label for="bepinex-url">BepInEx Download URL</Label>
						<Input
							id="bepinex-url"
							bind:value={localBepInExUrl}
							placeholder="https://builds.bepinex.dev/..."
						/>
					</div>

					<div class="flex items-center justify-between">
						<div class="space-y-0.5">
							<Label for="cache-bepinex">Keep Local Copy</Label>
							<p class="text-sm text-muted-foreground">
								Cache BepInEx locally for faster profile creation
							</p>
						</div>
						<Switch id="cache-bepinex" bind:checked={localCacheBepInEx} />
					</div>

					{#if localCacheBepInEx}
						<div class="flex items-center justify-between rounded-md bg-muted/50 p-3">
							<div class="space-y-0.5">
								<p class="text-sm font-medium">Cache Status</p>
								<p class="text-sm text-muted-foreground">
									{#if isCacheDownloading}
										Downloading... {cacheDownloadProgress.toFixed(1)}%
									{:else if isCacheExists}
										<span class="text-green-500">Cached</span>
									{:else}
										<span class="text-orange-500">Not cached</span>
									{/if}
								</p>
							</div>
							<div class="flex gap-2">
								<Button
									variant="outline"
									size="sm"
									onclick={handleDownloadToCache}
									disabled={isCacheDownloading}
								>
									{#if isCacheDownloading}
										<RefreshCw class="mr-2 h-4 w-4 animate-spin" />
										Downloading
									{:else}
										<Download class="mr-2 h-4 w-4" />
										{isCacheExists ? 'Re-download' : 'Download'}
									{/if}
								</Button>
								{#if isCacheExists}
									<Button variant="outline" size="sm" onclick={handleClearCache}>
										<Trash2 class="mr-2 h-4 w-4" />
										Clear
									</Button>
								{/if}
							</div>
						</div>
					{/if}
				</div>
			</div>

			<!-- App Behavior Section -->
			<div class="rounded-xl border border-border/50 bg-card/30 p-6 backdrop-blur-sm">
				<div class="mb-4 flex items-center justify-between">
					<h2 class="text-lg font-semibold tracking-tight">App Behavior</h2>
					{#if appBehaviorSaved}
						<span
							class="flex animate-in items-center gap-1.5 text-xs font-medium text-green-500 duration-200 fade-in"
						>
							<Check size={14} />
							Saved
						</span>
					{/if}
				</div>
				<div class="flex items-center justify-between">
					<div class="space-y-0.5">
						<Label for="close-on-launch">Close on Launch</Label>
						<p class="text-sm text-muted-foreground">
							Automatically close launcher after launching game
						</p>
					</div>
					<Switch id="close-on-launch" bind:checked={localCloseOnLaunch} />
				</div>
			</div>

			<!-- About Section -->
			<div class="lg:col-span-2">
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

					<div
						class="relative z-10 flex flex-col items-center text-center sm:flex-row sm:text-left"
					>
						<!-- Info content -->
						<div class="flex-1">
							<div class="mb-3 flex flex-col items-center gap-2 sm:flex-row sm:items-baseline">
								<h2 class="text-xl font-semibold">{APP_NAME}</h2>
								<span
									class="rounded-full bg-muted px-2.5 py-0.5 text-xs font-medium text-muted-foreground"
								>
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
			</div>
		</div>
	{/if}
</div>

<EpicLoginDialog bind:open={epicLoginOpen} onChange={refreshEpicAuth} />

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

	:global(.star) {
		will-change: opacity;
	}
</style>
