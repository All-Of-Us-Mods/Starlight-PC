<script lang="ts">
	import { updateState } from '$lib/features/updates/state/update-state.svelte';

	function formatBytes(bytes: number): string {
		if (bytes < 1024) return `${bytes} B`;
		const units = ['KB', 'MB', 'GB', 'TB'];
		let value = bytes / 1024;
		let unitIndex = 0;

		while (value >= 1024 && unitIndex < units.length - 1) {
			value /= 1024;
			unitIndex += 1;
		}

		return `${value.toFixed(value >= 10 ? 0 : 1)} ${units[unitIndex]}`;
	}

	const isInstalling = $derived(updateState.status === 'installing');
	const percent = $derived(updateState.progress.percent);
	const downloadedLabel = $derived(formatBytes(updateState.progress.downloaded));
	const totalLabel = $derived(
		updateState.progress.total ? formatBytes(updateState.progress.total) : null
	);
	const progressLabel = $derived(
		totalLabel ? `${downloadedLabel} / ${totalLabel}` : downloadedLabel
	);
</script>

<div class="flex min-w-0 flex-col gap-2">
	<div class="flex items-center justify-between gap-3">
		<p class="text-sm font-medium text-foreground">
			{isInstalling ? 'Installing update...' : 'Downloading update...'}
		</p>
		{#if !isInstalling}
			<span class="shrink-0 text-xs text-muted-foreground tabular-nums">{percent}%</span>
		{/if}
	</div>

	{#if !isInstalling}
		<div class="h-1.5 w-full overflow-hidden rounded-full bg-muted">
			<div
				class="h-full bg-primary transition-[width] duration-150"
				style="width: {percent}%"
			></div>
		</div>
		<p class="text-xs text-muted-foreground tabular-nums">{progressLabel}</p>
	{:else}
		<p class="text-xs text-muted-foreground">Preparing restart...</p>
	{/if}
</div>
