<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { Label } from '$lib/components/ui/label';
	import { Switch } from '$lib/components/ui/switch';
	import { Skeleton } from '$lib/components/ui/skeleton';
	import { Settings, Save, RefreshCw, Download, Trash2 } from '@lucide/svelte';
	import { createQuery, useQueryClient } from '@tanstack/svelte-query';
	import { settingsQueries } from '$lib/features/settings/queries';
	import { settingsService } from '$lib/features/settings/settings-service';
	import type { AppSettings, GamePlatform } from '$lib/features/settings/schema';
	import { showError, showSuccess } from '$lib/utils/toast';
	import { invoke } from '@tauri-apps/api/core';
	import { open as openDialog } from '@tauri-apps/plugin-dialog';
	import { exists } from '@tauri-apps/plugin-fs';
	import { listen, type UnlistenFn } from '@tauri-apps/api/event';
	import EpicLoginDialog from '$lib/features/settings/components/EpicLoginDialog.svelte';
	import { epicService } from '$lib/features/settings/epic-service';
	import type { DownloadProgress } from '$lib/features/profiles/bepinex-download';

	const settingsQuery = createQuery(() => settingsQueries.get());
	const settings = $derived(settingsQuery.data as AppSettings | undefined);
	const queryClient = useQueryClient();

	let isLoggedIn = $state(false);
	let isSaving = $state(false);
	let isDetecting = $state(false);
	let epicLoginOpen = $state(false);
	let isCacheDownloading = $state(false);
	let cacheDownloadProgress = $state(0);
	let isCacheExists = $state(false);

	async function refreshEpicAuth() {
		isLoggedIn = await epicService.isLoggedIn();
	}

	async function checkCacheExists() {
		try {
			const cachePath = await settingsService.getBepInExCachePath();
			isCacheExists = await invoke<boolean>('check_bepinex_cache_exists', { cachePath });
		} catch {
			isCacheExists = false;
		}
	}

	let localAmongUsPath = $state('');
	let localBepInExUrl = $state('');
	let localCloseOnLaunch = $state(false);
	let localGamePlatform = $state<GamePlatform>('steam');
	let localCacheBepInEx = $state(false);

	$effect(() => {
		if (settings) {
			localAmongUsPath = settings.among_us_path ?? '';
			localBepInExUrl = settings.bepinex_url ?? '';
			localCloseOnLaunch = settings.close_on_launch ?? false;
			localGamePlatform = settings.game_platform ?? 'steam';
			localCacheBepInEx = settings.cache_bepinex ?? false;
			refreshEpicAuth();
			checkCacheExists();
		}
	});

	async function handleSave() {
		if (localAmongUsPath) {
			const exePath = `${localAmongUsPath}/Among Us.exe`;
			if (!(await exists(exePath))) {
				showError('Selected folder does not contain Among Us.exe');
				return;
			}
		}

		isSaving = true;
		try {
			await settingsService.updateSettings({
				among_us_path: localAmongUsPath,
				bepinex_url: localBepInExUrl,
				close_on_launch: localCloseOnLaunch,
				game_platform: localGamePlatform,
				cache_bepinex: localCacheBepInEx
			});
			await handleAutoSetBepinex();
			queryClient.invalidateQueries({ queryKey: ['settings'] });
			showSuccess('Settings saved successfully');
		} catch (e) {
			showError(e);
		} finally {
			isSaving = false;
		}
	}

	async function handleAutoDetect() {
		isDetecting = true;
		try {
			const path = await invoke<string | null>('detect_among_us');
			if (path) {
				localAmongUsPath = path;
				const platform = await invoke<string>('get_game_platform', { path });
				localGamePlatform = platform as GamePlatform;
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

	async function handleAutoSetBepinex() {
		const crashHandlerPath = `${localAmongUsPath}/UnityCrashHandler64.exe`;

		if (await exists(crashHandlerPath)) {
			const url = (await settingsService.getSettings()).bepinex_url;
			const updatedurl = url.replace('x86', 'x64');
			await settingsService.updateSettings({ bepinex_url: updatedurl });
		} else {
			const url = (await settingsService.getSettings()).bepinex_url;
			const updatedurl = url.replace('x64', 'x86');
			await settingsService.updateSettings({ bepinex_url: updatedurl });
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
			}
		} catch (e) {
			showError(e);
		}
	}

	async function handleDownloadToCache() {
		if (!localBepInExUrl) {
			showError('BepInEx URL is required');
			return;
		}

		isCacheDownloading = true;
		cacheDownloadProgress = 0;
		let unlisten: UnlistenFn | undefined;

		try {
			unlisten = await listen<DownloadProgress>('download-progress', (event) => {
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
</script>

<div class="px-10 py-8">
	<div class="mb-6 flex items-center gap-3">
		<div
			class="flex h-12 w-12 items-center justify-center rounded-xl bg-primary/10 ring-1 ring-primary/20"
		>
			<Settings class="h-6 w-6 text-primary" />
		</div>
		<div class="space-y-0.5">
			<h1 class="text-4xl font-black tracking-tight">Settings</h1>
			<p class="text-sm text-muted-foreground">Configure your Among Us path and app preferences.</p>
		</div>
	</div>

	{#if settingsQuery.isPending}
		<div class="max-w-2xl space-y-6">
			<div class="space-y-4 rounded-lg border border-border p-6">
				<Skeleton class="h-6 w-1/3" />
				<Skeleton class="h-10 w-full" />
				<Skeleton class="h-4 w-2/3" />
			</div>
			<div class="space-y-4 rounded-lg border border-border p-6">
				<Skeleton class="h-6 w-1/3" />
				<Skeleton class="h-10 w-full" />
				<Skeleton class="h-10 w-full" />
			</div>
			<div class="rounded-lg border border-border p-6">
				<Skeleton class="h-6 w-1/3" />
				<div class="mt-4 flex items-center justify-between">
					<div class="space-y-2">
						<Skeleton class="h-4 w-32" />
						<Skeleton class="h-3 w-48" />
					</div>
					<Skeleton class="h-6 w-12" />
				</div>
			</div>
		</div>
	{:else}
		<div class="max-w-2xl space-y-6">
			<div class="rounded-lg border border-border p-6">
				<h2 class="mb-4 text-lg font-semibold">Game Configuration</h2>
				<div class="space-y-4">
					<div class="space-y-2">
						<Label for="among-us-path">Among Us Installation Path</Label>
						<div class="flex gap-2">
							<Input
								id="among-us-path"
								bind:value={localAmongUsPath}
								placeholder="C:\Program Files (x86)\Steam\steamapps\common\Among Us"
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
					</div>

					<div class="space-y-2">
						<Label for="game-platform">Game Platform</Label>
						<div class="flex gap-2">
							<Button
								variant={localGamePlatform === 'steam' ? 'default' : 'outline'}
								onclick={() => (localGamePlatform = 'steam')}
								class="flex-1"
							>
								Steam
							</Button>
							<Button
								variant={localGamePlatform === 'epic' ? 'default' : 'outline'}
								onclick={() => (localGamePlatform = 'epic')}
								class="flex-1"
							>
								Epic Games
							</Button>
						</div>
						<p class="text-sm text-muted-foreground">
							{#if localGamePlatform === 'steam'}
								Steam installation
							{:else}
								Epic Games installation (requires Epic Games login)
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

			<div class="rounded-lg border border-border p-6">
				<h2 class="mb-4 text-lg font-semibold">BepInEx Configuration</h2>
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

			<div class="rounded-lg border border-border p-6">
				<h2 class="mb-4 text-lg font-semibold">App Behavior</h2>
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

			<div class="flex justify-end gap-2">
				<Button onclick={handleSave} disabled={isSaving}>
					{#if isSaving}
						Saving...
					{:else}
						<Save class="mr-2 h-4 w-4" />
						Save Settings
					{/if}
				</Button>
			</div>
		</div>
	{/if}
</div>

<EpicLoginDialog bind:open={epicLoginOpen} onChange={refreshEpicAuth} />
