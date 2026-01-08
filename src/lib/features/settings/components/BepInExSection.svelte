<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { Label } from '$lib/components/ui/label';
	import { Switch } from '$lib/components/ui/switch';
	import { RefreshCw, Download, Trash2 } from '@jis3r/icons';
	import { showError, showSuccess } from '$lib/utils/toast';
	import { invoke } from '@tauri-apps/api/core';
	import { listen, type UnlistenFn } from '@tauri-apps/api/event';
	import { settingsService } from '../settings-service';
	import type { BepInExProgress } from '$lib/features/profiles/bepinex-download';

	let {
		bepInExUrl = $bindable(''),
		cacheBepInEx = $bindable(false),
		isCacheExists = $bindable(false)
	}: {
		bepInExUrl: string;
		cacheBepInEx: boolean;
		isCacheExists: boolean;
	} = $props();

	let isCacheDownloading = $state(false);
	let cacheDownloadProgress = $state(0);

	async function handleDownloadToCache() {
		if (!bepInExUrl) {
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
				url: bepInExUrl,
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

<div class="rounded-xl border border-border/50 bg-card/30 p-6 backdrop-blur-sm">
	<h2 class="mb-4 text-lg font-semibold tracking-tight">BepInEx Configuration</h2>
	<div class="space-y-4">
		<div class="space-y-2">
			<Label for="bepinex-url">BepInEx Download URL</Label>
			<Input
				id="bepinex-url"
				bind:value={bepInExUrl}
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
			<Switch id="cache-bepinex" bind:checked={cacheBepInEx} />
		</div>

		{#if cacheBepInEx}
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
