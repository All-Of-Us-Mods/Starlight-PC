<script lang="ts">
	import * as Card from '$lib/components/ui/card';
	import { Badge } from '$lib/components/ui/badge';
	import { ImageOff, Trash } from '@jis3r/icons';
	import type { Mod } from '$lib/features/mods/schema';
	import type { ProfileMod } from '$lib/features/profiles/schema';
	import Button from '$lib/components/ui/button/button.svelte';
	import ModDetailsSidebar from './ModDetailsSidebar.svelte';
	import { getSidebar } from '$lib/state/sidebar.svelte';
	import { rememberInstallTarget } from '$lib/features/mods/state/install-target.svelte';
	import { Download, LoaderCircle } from '@lucide/svelte';

	interface Props {
		mod: Mod;
		profileMod: ProfileMod;
		profileId: string;
		latestVersion?: string;
		isOutdated?: boolean;
		isCheckingUpdate?: boolean;
		isUpdating?: boolean;
		onUpdate?: () => void;
		ondelete?: () => void;
	}

	let {
		mod,
		profileMod,
		profileId,
		latestVersion,
		isOutdated = false,
		isCheckingUpdate = false,
		isUpdating = false,
		onUpdate,
		ondelete
	}: Props = $props();

	const sidebar = getSidebar();
	const contentId = $derived(`profile-${profileId}-mod-${mod.id}`);
	const isSelected = $derived(sidebar.contentId === contentId);

	function openModDetails() {
		rememberInstallTarget(profileId, 'profile-context');
		sidebar.open(sidebarContent, undefined, contentId);
	}

	function handleDelete() {
		ondelete?.();
	}

	function handleUpdate() {
		onUpdate?.();
	}
</script>

{#snippet sidebarContent()}
	<ModDetailsSidebar modId={mod.id} {profileId} onclose={() => sidebar.close()} />
{/snippet}

<Card.Root
	class="relative overflow-hidden p-0 transition-all hover:bg-accent/50 {isSelected
		? 'border-primary ring-1 ring-primary'
		: 'hover:border-primary/50'}"
>
	<button
		type="button"
		onclick={openModDetails}
		class="flex w-full cursor-pointer gap-3 p-3 text-left"
	>
		<div
			class="flex h-16 w-16 flex-none items-center justify-center overflow-hidden rounded-lg bg-muted"
		>
			{#if mod._links.thumbnail}
				<img src={mod._links.thumbnail} alt={mod.name} class="h-full w-full object-cover" />
			{:else}
				<ImageOff class="h-10 w-10 text-muted-foreground/40" />
			{/if}
		</div>

		<div class="min-w-0 flex-1 pr-28">
			<div class="mb-0.5 flex items-center gap-2">
				<h3 class="truncate text-base font-bold" title={mod.name}>
					{mod.name}
				</h3>
				<Badge variant="secondary" class="shrink-0 text-[10px]">v{profileMod.version}</Badge>
				{#if isOutdated}
					<Badge variant="default" class="shrink-0 text-[10px]">Update available</Badge>
				{/if}
			</div>
			<p class="mb-1 truncate text-sm text-muted-foreground/80">
				by {mod.author}
			</p>
			{#if isOutdated && latestVersion}
				<p class="mb-1 text-xs text-primary">v{profileMod.version} -&gt; v{latestVersion}</p>
			{/if}
			{#if isCheckingUpdate}
				<p class="mb-1 text-xs text-muted-foreground">Checking for updates...</p>
			{/if}
			<p class="line-clamp-2 text-sm text-muted-foreground">
				{mod.description}
			</p>
		</div>
	</button>

	<div class="absolute top-3 right-3 flex items-center gap-2">
		{#if isOutdated && latestVersion}
			<Button
				size="sm"
				variant="secondary"
				class="flex-none"
				onclick={handleUpdate}
				disabled={isUpdating}
			>
				{#if isUpdating}
					<LoaderCircle class="size-4 animate-spin" />
				{:else}
					<Download class="size-4" />
				{/if}
			</Button>
		{/if}
		<Button size="sm" variant="destructive" class="flex-none" onclick={handleDelete}>
			<Trash class="size-4" />
		</Button>
	</div>
</Card.Root>
