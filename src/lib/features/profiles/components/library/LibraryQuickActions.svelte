<script lang="ts" module>
	export interface LibraryQuickActionsProps {
		isLaunchingVanilla: boolean;
		onLaunchVanilla: () => void | Promise<void>;
	}
</script>

<script lang="ts">
	import { Ghost, Play } from '@lucide/svelte';

	let { isLaunchingVanilla, onLaunchVanilla }: LibraryQuickActionsProps = $props();
</script>

<div class="mb-6">
	<h2 class="mb-3 text-lg font-semibold">Quick Actions</h2>
	<div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
		<button
			onclick={onLaunchVanilla}
			disabled={isLaunchingVanilla}
			class="flex items-center gap-3 rounded-lg border border-border bg-muted/20 p-4 text-left transition-colors hover:bg-accent/50 disabled:cursor-not-allowed disabled:opacity-50"
		>
			<div class="flex h-10 w-10 shrink-0 items-center justify-center rounded-full bg-primary/10">
				{#if isLaunchingVanilla}
					<div
						class="h-5 w-5 animate-spin rounded-full border-2 border-primary border-t-transparent"
					></div>
				{:else}
					<Ghost class="h-5 w-5 text-primary" />
				{/if}
			</div>
			<div class="flex min-w-0 flex-1 flex-col">
				<div class="flex items-center gap-2">
					<div class="font-semibold">{isLaunchingVanilla ? 'Launching...' : 'Launch Vanilla'}</div>
				</div>
				<div class="truncate text-sm text-muted-foreground">Play without any mods</div>
			</div>
			{#if !isLaunchingVanilla}
				<Play class="ml-2 h-5 w-5 shrink-0" />
			{/if}
		</button>
	</div>
</div>
