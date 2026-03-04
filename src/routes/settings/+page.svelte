<script lang="ts">
	import { Skeleton } from '$lib/components/ui/skeleton';
	import PageHeader from '$lib/components/shared/PageHeader.svelte';
	import { Settings } from '@jis3r/icons';
	import { createQuery, createMutation, useQueryClient } from '@tanstack/svelte-query';
	import { settingsQueries } from '$lib/features/settings/queries';
	import { settingsMutations } from '$lib/features/settings/mutations';
	import type { AppSettings, GamePlatform } from '$lib/features/settings/schema';
	import { showError, showSuccess } from '$lib/utils/toast';
	import { exists } from '@tauri-apps/plugin-fs';
	import { open as openDialog } from '@tauri-apps/plugin-dialog';
	import { error as logError } from '@tauri-apps/plugin-log';
	import { epicService } from '$lib/features/settings/epic-service';
	import EpicLoginDialog from '$lib/components/settings/EpicLoginDialog.svelte';
	import { Debounced, watch } from 'runed';
	import GameSettingsSection from './_components/GameSettingsSection.svelte';
	import BepInExSettingsSection from './_components/BepInExSettingsSection.svelte';
	import AppBehaviorSection from './_components/AppBehaviorSection.svelte';
	import AboutStarlightCard from './_components/AboutStarlightCard.svelte';

	const GITHUB_URL = 'https://github.com/All-Of-Us-Mods/Starlight-PC';

	const queryClient = useQueryClient();
	const settingsQuery = createQuery(() => settingsQueries.get());
	const settings = $derived(settingsQuery.data as AppSettings | undefined);
	const updateMutation = createMutation(() => settingsMutations.update(queryClient));
	const detectBepInExMutation = createMutation(() =>
		settingsMutations.autoDetectBepInExArchitecture(queryClient)
	);
	const downloadCacheMutation = createMutation(() => settingsMutations.downloadBepInExToCache());
	const clearCacheMutation = createMutation(() => settingsMutations.clearBepInExCache());
	const openDataFolderMutation = createMutation(() => settingsMutations.openDataFolder());
	const checkCacheExistsMutation = createMutation(() => settingsMutations.checkCacheExists());
	const detectAmongUsPathMutation = createMutation(() => settingsMutations.detectAmongUsPath());
	const detectGameStoreMutation = createMutation(() => settingsMutations.detectGameStore());

	// Form state
	let localPath = $state('');
	let localUrl = $state('');
	let localPlatform = $state<GamePlatform>('steam');
	let localCacheBepInEx = $state(false);
	let localCloseOnLaunch = $state(false);
	let localAllowMultiInstanceLaunch = $state(false);

	// UI state
	let initialized = $state(false);
	let isHydrating = $state(true);
	let pathError = $state('');
	let isLoggedIn = $state(false);
	let isCacheExists = $state(false);
	let isDetecting = $state(false);
	let epicLoginOpen = $state(false);
	let isCacheDownloading = $state(false);
	let cacheDownloadProgress = $state(0);
	const debouncedPath = new Debounced(() => localPath, 500);
	const debouncedUrl = new Debounced(() => localUrl, 500);

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
			const newUrl = await detectBepInExMutation.mutateAsync(path);
			if (newUrl) localUrl = newUrl;
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
		if (!localUrl) return showError('BepInEx URL is required');
		isCacheDownloading = true;
		cacheDownloadProgress = 0;
		try {
			await downloadCacheMutation.mutateAsync({
				url: localUrl,
				onProgress: (progress) => {
				cacheDownloadProgress = progress.progress;
				}
			});
			isCacheExists = true;
			showSuccess('BepInEx downloaded to cache');
		} catch (e) {
			showError(e);
		} finally {
			isCacheDownloading = false;
			cacheDownloadProgress = 0;
		}
	}

	async function handleClearCache() {
		try {
			await clearCacheMutation.mutateAsync();
			isCacheExists = false;
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
			localUrl = settings.bepinex_url ?? '';
			localCloseOnLaunch = settings.close_on_launch ?? false;
			localAllowMultiInstanceLaunch = settings.allow_multi_instance_launch ?? false;
			localPlatform = settings.game_platform ?? 'steam';
			localCacheBepInEx = settings.cache_bepinex ?? false;
			epicService.isLoggedIn().then((v) => (isLoggedIn = v));
			checkCacheExistsMutation.mutateAsync().then((v) => (isCacheExists = v));
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
				void updateMutation.mutateAsync({ xbox_app_id: undefined });
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
				void updateMutation.mutateAsync({ xbox_app_id: undefined });
			}
		},
		{ lazy: true }
	);

	// Debounced URL save to avoid writing on every keystroke.
	watch(
		() => debouncedUrl.current,
		() => {
			if (isHydrating) return;
			void updateMutation.mutateAsync({ bepinex_url: localUrl });
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
				bind:localUrl
				bind:localCacheBepInEx
				{isCacheDownloading}
				{cacheDownloadProgress}
				{isCacheExists}
				onDownloadToCache={handleDownloadToCache}
				onClearCache={handleClearCache}
			/>
			<AppBehaviorSection bind:localCloseOnLaunch bind:localAllowMultiInstanceLaunch />
			<AboutStarlightCard githubUrl={GITHUB_URL} onOpenDataFolder={handleOpenDataFolder} />
		</div>
	{/if}
</div>

<EpicLoginDialog
	bind:open={epicLoginOpen}
	onChange={() => epicService.isLoggedIn().then((v) => (isLoggedIn = v))}
/>
