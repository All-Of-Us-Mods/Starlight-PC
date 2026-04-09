<script lang="ts">
	import { Skeleton } from '$lib/components/ui/skeleton';
	import PageHeader from '$lib/components/shared/PageHeader.svelte';
	import { Settings } from '@jis3r/icons';
	import { createQuery, createMutation, useQueryClient } from '@tanstack/svelte-query';
	import { listen } from '@tauri-apps/api/event';
	import { settingsQueries } from '$lib/features/settings/queries';
	import { settingsActions } from '$lib/features/settings/actions';
	import { settingsCacheExistsQueryKey } from '$lib/features/settings/settings-keys';
	import type { AppSettings, GamePlatform } from '$lib/features/settings/schema';
	import type { BepInExProgress } from '$lib/features/profiles/schema';
	import { showError, showSuccess } from '$lib/utils/toast';
	import { exists } from '@tauri-apps/plugin-fs';
	import { open as openDialog } from '@tauri-apps/plugin-dialog';
	import { error as logError } from '@tauri-apps/plugin-log';
	import { epicAuthService } from '$lib/features/settings/services/epic-auth.service';
	import EpicLoginDialogContainer from '$lib/features/settings/containers/EpicLoginDialogContainer.svelte';
	import { Debounced, watch } from 'runed';
	import GameSettingsSection from '$lib/features/settings/components/GameSettingsSection.svelte';
	import BepInExSettingsSection from '$lib/features/settings/components/BepInExSettingsSection.svelte';
	import AppBehaviorSection from '$lib/features/settings/components/AppBehaviorSection.svelte';
	import AboutStarlightCardContainer from '$lib/features/settings/containers/AboutStarlightCardContainer.svelte';

	const GITHUB_URL = 'https://github.com/All-Of-Us-Mods/Starlight-PC';

	const queryClient = useQueryClient();
	const settingsQuery = createQuery(() => settingsQueries.get());
	const settings = $derived(settingsQuery.data as AppSettings | undefined);
	const updateMutation = createMutation(() => settingsActions.update(queryClient));
	const downloadCacheMutation = createMutation(() => settingsActions.downloadBepInExToCache());
	const clearCacheMutation = createMutation(() => settingsActions.clearBepInExCache());
	const openDataFolderMutation = createMutation(() => settingsActions.openDataFolder());
	const detectAmongUsPathMutation = createMutation(() => settingsActions.detectAmongUsPath());
	const detectGameStoreMutation = createMutation(() => settingsActions.detectGameStore());

	// Form state
	let localPath = $state('');
	let localUrlX86 = $state('');
	let localUrlX64 = $state('');
	let localPlatform = $state<GamePlatform>('steam');
	let localCacheBepInEx = $state(false);
	let localCloseOnLaunch = $state(false);
	let localAllowMultiInstanceLaunch = $state(false);
	const activeBepInExArch = $derived(
		localPlatform === 'epic' || localPlatform === 'xbox' ? 'x64' : 'x86'
	);
	const cacheExistsQuery = createQuery(() => settingsQueries.cacheExists(activeBepInExArch));

	// UI state
	let initialized = $state(false);
	let isHydrating = $state(true);
	let pathError = $state('');
	let isLoggedIn = $state(false);
	const isCacheExists = $derived((cacheExistsQuery.data as boolean | undefined) ?? false);
	let isDetecting = $state(false);
	let epicLoginOpen = $state(false);
	let isCacheDownloading = $state(false);
	let cacheDownloadProgress = $state(0);
	const debouncedPath = new Debounced(() => localPath, 500);
	const debouncedUrlX86 = new Debounced(() => localUrlX86, 500);
	const debouncedUrlX64 = new Debounced(() => localUrlX64, 500);

	async function validatePath(path: string): Promise<boolean> {
		if (!path) return ((pathError = ''), true);
		if (!(await exists(`${path}/Among Us.exe`))) {
			pathError = 'Selected folder does not contain Among Us.exe';
			return false;
		}
		return ((pathError = ''), true);
	}

	async function saveGameConfig(path: string, platform: GamePlatform) {
		const valid = await validatePath(path);
		if (!valid) return;

		try {
			await updateMutation.mutateAsync({ among_us_path: path, game_platform: platform });
		} catch (e) {
			showError(e);
		}
	}

	async function saveAppBehavior() {
		try {
			await updateMutation.mutateAsync({ close_on_launch: localCloseOnLaunch });
		} catch (e) {
			showError(e);
		}
	}

	async function detectPlatform(path: string) {
		try {
			localPlatform = await detectGameStoreMutation.mutateAsync(path);
		} catch (e) {
			logError(`Platform detection failed: ${e}`);
		}
	}

	async function handleAutoDetect() {
		isDetecting = true;
		try {
			const path = await detectAmongUsPathMutation.mutateAsync();
			if (path) {
				localPath = path;
				await detectPlatform(path);
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
				localPath = selected;
				await detectPlatform(selected);
			}
		} catch (e) {
			showError(e);
		}
	}

	async function handleDownloadToCache() {
		const url = activeBepInExArch === 'x64' ? localUrlX64 : localUrlX86;
		if (!url) return showError('BepInEx URL is required');
		isCacheDownloading = true;
		cacheDownloadProgress = 0;
		let unlisten: (() => void) | undefined;
		try {
			unlisten = await listen<BepInExProgress>('bepinex-progress', (event) => {
				cacheDownloadProgress = event.payload.progress;
			});
			await downloadCacheMutation.mutateAsync({ url, architecture: activeBepInExArch });
			queryClient.setQueryData(settingsCacheExistsQueryKey(activeBepInExArch), true);
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
			await clearCacheMutation.mutateAsync(activeBepInExArch);
			queryClient.setQueryData(settingsCacheExistsQueryKey(activeBepInExArch), false);
			showSuccess('Cache cleared');
		} catch (e) {
			showError(e);
		}
	}

	async function handleOpenDataFolder() {
		try {
			await openDataFolderMutation.mutateAsync();
		} catch (e) {
			showError(e, 'Open data folder');
		}
	}

	// Initialize state from settings
	$effect(() => {
		if (settings && !initialized) {
			localPath = settings.among_us_path ?? '';
			localUrlX86 = settings.bepinex_url_x86 ?? '';
			localUrlX64 = settings.bepinex_url_x64 ?? '';
			localCloseOnLaunch = settings.close_on_launch ?? false;
			localAllowMultiInstanceLaunch = settings.allow_multi_instance_launch ?? false;
			localPlatform = settings.game_platform ?? 'steam';
			localCacheBepInEx = settings.cache_bepinex ?? false;
			epicAuthService.isLoggedIn().then((v) => (isLoggedIn = v));
			initialized = true;
			isHydrating = false;
		}
	});

	// Debounced path save to avoid writing on every keystroke.
	watch(
		() => debouncedPath.current,
		(newPath, oldPath) => {
			if (isHydrating) return;
			void saveGameConfig(newPath, localPlatform);

			// Path controls the Xbox identity context; clear stale app id on edits.
			if (newPath !== oldPath && settings?.xbox_app_id) {
				void updateMutation.mutateAsync({ xbox_app_id: null });
			}
		},
		{ lazy: true }
	);

	// Save platform changes immediately.
	watch(
		() => localPlatform,
		(newPlatform, oldPlatform) => {
			if (isHydrating) return;
			void updateMutation.mutateAsync({ game_platform: newPlatform });

			if (newPlatform !== oldPlatform && settings?.xbox_app_id) {
				void updateMutation.mutateAsync({ xbox_app_id: null });
			}
		},
		{ lazy: true }
	);

	// Debounced URL save to avoid writing on every keystroke.
	watch(
		() => debouncedUrlX86.current,
		() => {
			if (isHydrating) return;
			void updateMutation.mutateAsync({ bepinex_url_x86: localUrlX86 });
		},
		{ lazy: true }
	);

	watch(
		() => debouncedUrlX64.current,
		() => {
			if (isHydrating) return;
			void updateMutation.mutateAsync({ bepinex_url_x64: localUrlX64 });
		},
		{ lazy: true }
	);

	// Save cache toggle immediately.
	watch(
		() => localCacheBepInEx,
		() => {
			if (isHydrating) return;
			void updateMutation.mutateAsync({ cache_bepinex: localCacheBepInEx });
		},
		{ lazy: true }
	);

	// Save app behavior immediately.
	watch(
		() => localCloseOnLaunch,
		() => {
			if (isHydrating) return;
			void saveAppBehavior();
		},
		{ lazy: true }
	);

	watch(
		() => localAllowMultiInstanceLaunch,
		() => {
			if (isHydrating) return;
			void updateMutation.mutateAsync({
				allow_multi_instance_launch: localAllowMultiInstanceLaunch
			});
		},
		{ lazy: true }
	);
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
			<GameSettingsSection
				bind:localPath
				bind:localPlatform
				{pathError}
				{isDetecting}
				{isLoggedIn}
				onBrowse={handleBrowse}
				onAutoDetect={handleAutoDetect}
				onOpenEpicLogin={() => (epicLoginOpen = true)}
				onPathBlur={() => validatePath(localPath)}
			/>
			<BepInExSettingsSection
				bind:localUrlX86
				bind:localUrlX64
				activeArchitecture={activeBepInExArch}
				bind:localCacheBepInEx
				{isCacheDownloading}
				{cacheDownloadProgress}
				{isCacheExists}
				onDownloadToCache={handleDownloadToCache}
				onClearCache={handleClearCache}
			/>
			<AppBehaviorSection bind:localCloseOnLaunch bind:localAllowMultiInstanceLaunch />
			<AboutStarlightCardContainer githubUrl={GITHUB_URL} onOpenDataFolder={handleOpenDataFolder} />
		</div>
	{/if}
</div>

<EpicLoginDialogContainer
	bind:open={epicLoginOpen}
	onChange={() => epicAuthService.isLoggedIn().then((v) => (isLoggedIn = v))}
/>
