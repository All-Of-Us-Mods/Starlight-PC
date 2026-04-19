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
	import { platform } from '@tauri-apps/plugin-os';
	import { Input } from '$lib/components/ui/input';
	import { Label } from '$lib/components/ui/label';
	import { Button } from '$lib/components/ui/button';
	import { Switch } from '$lib/components/ui/switch';
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
	const detectLinuxRunnerMutation = createMutation(() => settingsActions.detectLinuxRunner());

	// Form state
	let localPath = $state('');
	let localUrlX86 = $state('');
	let localUrlX64 = $state('');
	let localPlatform = $state<GamePlatform>('steam');
	let localCacheBepInEx = $state(false);
	let localCloseOnLaunch = $state(false);
	let localAllowMultiInstanceLaunch = $state(false);
	let localLinuxRunnerKind = $state<'wine' | 'proton'>('proton');
	let localLinuxRunnerBinary = $state('');
	let localLinuxWinePrefix = $state('');
	let localLinuxProtonCompatDataPath = $state('');
	let localLinuxProtonSteamClientPath = $state('');
	let localLinuxProtonUseSteamRun = $state(true);
	const isLinux = platform() === 'linux';
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
	const debouncedLinuxRunnerBinary = new Debounced(() => localLinuxRunnerBinary, 500);
	const debouncedLinuxWinePrefix = new Debounced(() => localLinuxWinePrefix, 500);
	const debouncedLinuxProtonCompatDataPath = new Debounced(() => localLinuxProtonCompatDataPath, 500);
	const debouncedLinuxProtonSteamClientPath = new Debounced(() => localLinuxProtonSteamClientPath, 500);

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
				if (isLinux) {
					await handleDetectLinuxRunner(path);
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
				localPath = selected;
				await detectPlatform(selected);
				if (isLinux) {
					await handleDetectLinuxRunner(selected);
				}
			}
		} catch (e) {
			showError(e);
		}
	}

	async function handleDetectLinuxRunner(path?: string) {
		if (!isLinux) return;
		try {
			const detected = await detectLinuxRunnerMutation.mutateAsync(path);
			localLinuxRunnerKind = detected.runnerKind;
			localLinuxRunnerBinary = detected.runnerBinary ?? '';
			localLinuxWinePrefix = detected.winePrefix ?? '';
			localLinuxProtonCompatDataPath = detected.protonCompatDataPath ?? '';
			localLinuxProtonSteamClientPath = detected.protonSteamClientPath ?? '';
			localLinuxProtonUseSteamRun = detected.protonUseSteamRun;

			await updateMutation.mutateAsync({
				linux_runner_kind: detected.runnerKind,
				linux_runner_binary: detected.runnerBinary ?? '',
				linux_wine_prefix: detected.winePrefix ?? '',
				linux_proton_compat_data_path: detected.protonCompatDataPath ?? '',
				linux_proton_steam_client_path: detected.protonSteamClientPath ?? '',
				linux_proton_use_steam_run: detected.protonUseSteamRun
			});
		} catch (e) {
			showError(e, 'Linux runner detection');
		}
	}

	async function handleDownloadToCache() {
		const architecture = activeBepInExArch;
		const url = architecture === 'x64' ? localUrlX64 : localUrlX86;
		if (!url) return showError('BepInEx URL is required');
		isCacheDownloading = true;
		cacheDownloadProgress = 0;
		let unlisten: (() => void) | undefined;
		try {
			unlisten = await listen<BepInExProgress>('bepinex-progress', (event) => {
				if (
					event.payload.targetType !== 'cache' ||
					event.payload.targetId !== architecture
				)
					return;
				cacheDownloadProgress = event.payload.progress;
			});
			await downloadCacheMutation.mutateAsync({ url, architecture });
			queryClient.setQueryData(settingsCacheExistsQueryKey(architecture), true);
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
			localLinuxRunnerKind = settings.linux_runner_kind ?? 'proton';
			localLinuxRunnerBinary = settings.linux_runner_binary ?? '';
			localLinuxWinePrefix = settings.linux_wine_prefix ?? '';
			localLinuxProtonCompatDataPath = settings.linux_proton_compat_data_path ?? '';
			localLinuxProtonSteamClientPath = settings.linux_proton_steam_client_path ?? '';
			localLinuxProtonUseSteamRun = settings.linux_proton_use_steam_run ?? true;
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

	watch(
		() => localLinuxRunnerKind,
		() => {
			if (isHydrating || !isLinux) return;
			void updateMutation.mutateAsync({ linux_runner_kind: localLinuxRunnerKind });
		},
		{ lazy: true }
	);

	watch(
		() => debouncedLinuxRunnerBinary.current,
		() => {
			if (isHydrating || !isLinux) return;
			void updateMutation.mutateAsync({ linux_runner_binary: localLinuxRunnerBinary });
		},
		{ lazy: true }
	);

	watch(
		() => debouncedLinuxWinePrefix.current,
		() => {
			if (isHydrating || !isLinux) return;
			void updateMutation.mutateAsync({ linux_wine_prefix: localLinuxWinePrefix });
		},
		{ lazy: true }
	);

	watch(
		() => debouncedLinuxProtonCompatDataPath.current,
		() => {
			if (isHydrating || !isLinux) return;
			void updateMutation.mutateAsync({
				linux_proton_compat_data_path: localLinuxProtonCompatDataPath
			});
		},
		{ lazy: true }
	);

	watch(
		() => debouncedLinuxProtonSteamClientPath.current,
		() => {
			if (isHydrating || !isLinux) return;
			void updateMutation.mutateAsync({
				linux_proton_steam_client_path: localLinuxProtonSteamClientPath
			});
		},
		{ lazy: true }
	);

	watch(
		() => localLinuxProtonUseSteamRun,
		() => {
			if (isHydrating || !isLinux) return;
			void updateMutation.mutateAsync({ linux_proton_use_steam_run: localLinuxProtonUseSteamRun });
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
			{#if isLinux}
				<section class="space-y-4 rounded-xl border border-border/50 bg-card/30 p-6 backdrop-blur-sm lg:col-span-2">
					<header>
						<h2 class="text-lg font-semibold tracking-tight">Linux Runner</h2>
					</header>
					<div class="flex justify-end">
						<Button variant="outline" size="sm" onclick={() => handleDetectLinuxRunner(localPath)}>
							Auto-detect runner
						</Button>
					</div>
					<div class="space-y-2">
						<Label>Runner Type</Label>
						<div class="flex gap-2">
							<Button
								variant={localLinuxRunnerKind === 'proton' ? 'default' : 'outline'}
								onclick={() => (localLinuxRunnerKind = 'proton')}
								class="flex-1"
							>
								Proton
							</Button>
							<Button
								variant={localLinuxRunnerKind === 'wine' ? 'default' : 'outline'}
								onclick={() => (localLinuxRunnerKind = 'wine')}
								class="flex-1"
							>
								Wine
							</Button>
						</div>
					</div>

					<div class="space-y-2">
						<Label for="linux-runner-binary">Runner Binary</Label>
						<Input
							id="linux-runner-binary"
							bind:value={localLinuxRunnerBinary}
							placeholder={localLinuxRunnerKind === 'proton'
								? '/home/user/.local/share/Steam/steamapps/common/Proton - Experimental/proton'
								: 'wine'}
						/>
					</div>

					{#if localLinuxRunnerKind === 'proton'}
						<div class="flex items-center justify-between rounded-md bg-muted/50 p-3">
							<div class="space-y-0.5">
								<Label for="linux-proton-use-steam-run">Use steam-run</Label>
								<p class="text-sm text-muted-foreground">
									Wrap Proton with steam-run for Nix/FHS environments
								</p>
							</div>
							<Switch id="linux-proton-use-steam-run" bind:checked={localLinuxProtonUseSteamRun} />
						</div>

						<div class="space-y-2">
							<Label for="linux-proton-compat-data-path">Compat Data Path</Label>
							<Input
								id="linux-proton-compat-data-path"
								bind:value={localLinuxProtonCompatDataPath}
								placeholder="/mnt/games/SteamLibrary/steamapps/compatdata/945360"
							/>
						</div>
						<div class="space-y-2">
							<Label for="linux-proton-steam-client-path">Steam Client Path</Label>
							<Input
								id="linux-proton-steam-client-path"
								bind:value={localLinuxProtonSteamClientPath}
								placeholder="/home/user/.local/share/Steam"
							/>
						</div>
					{:else}
						<div class="space-y-2">
							<Label for="linux-wine-prefix">Wine Prefix</Label>
							<Input
								id="linux-wine-prefix"
								bind:value={localLinuxWinePrefix}
								placeholder="/home/user/.wine"
							/>
						</div>
					{/if}
				</section>
			{/if}
			<AppBehaviorSection bind:localCloseOnLaunch bind:localAllowMultiInstanceLaunch />
			<AboutStarlightCardContainer githubUrl={GITHUB_URL} onOpenDataFolder={handleOpenDataFolder} />
		</div>
	{/if}
</div>

<EpicLoginDialogContainer
	bind:open={epicLoginOpen}
	onChange={() => epicAuthService.isLoggedIn().then((v) => (isLoggedIn = v))}
/>
