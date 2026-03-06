<script lang="ts" module>
	export interface BepInExSettingsSectionProps {
		localUrl: string;
		localCacheBepInEx: boolean;
		isCacheDownloading: boolean;
		cacheDownloadProgress: number;
		isCacheExists: boolean;
		onDownloadToCache: () => void | Promise<void>;
		onClearCache: () => void | Promise<void>;
	}
</script>

<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { Label } from '$lib/components/ui/label';
	import { Switch } from '$lib/components/ui/switch';
	import { Download, RefreshCw, Trash2 } from '@jis3r/icons';
	import SettingsSection from './SettingsSection.svelte';

	let {
		localUrl = $bindable(),
		localCacheBepInEx = $bindable(),
		isCacheDownloading,
		cacheDownloadProgress,
		isCacheExists,
		onDownloadToCache,
		onClearCache
	}: BepInExSettingsSectionProps = $props();
</script>

<SettingsSection title="BepInEx Configuration">
	<div class="space-y-4">
		<div class="space-y-2">
			<Label for="bepinex-url">BepInEx Download URL</Label>
			<Input id="bepinex-url" bind:value={localUrl} placeholder="https://builds.bepinex.dev/..." />
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
						{:else}
							<span class:text-green-500={isCacheExists} class:text-orange-500={!isCacheExists}>
								{isCacheExists ? 'Cached' : 'Not cached'}
							</span>
						{/if}
					</p>
				</div>
				<div class="flex gap-2">
					<Button
						variant="outline"
						size="sm"
						onclick={onDownloadToCache}
						disabled={isCacheDownloading}
					>
						{#if isCacheDownloading}
							<RefreshCw class="mr-2 h-4 w-4 animate-spin" /> Downloading
						{:else}
							<Download class="mr-2 h-4 w-4" /> {isCacheExists ? 'Re-download' : 'Download'}
						{/if}
					</Button>
					{#if isCacheExists}
						<Button variant="outline" size="sm" onclick={onClearCache}>
							<Trash2 class="mr-2 h-4 w-4" /> Clear
						</Button>
					{/if}
				</div>
			</div>
		{/if}
	</div>
</SettingsSection>
