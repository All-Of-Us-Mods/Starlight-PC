<script lang="ts">
	import * as Card from '$lib/components/ui/card';
	import { Badge } from '$lib/components/ui/badge';
	import { ImageOff, Trash } from '@jis3r/icons';
	import type { Mod } from '$lib/features/mods/schema';
	import type { ProfileMod } from '$lib/features/profiles/schema';
	import Button from '$lib/components/ui/button/button.svelte';
	import ModDetailsSidebar from './ModDetailsSidebar.svelte';
	import { getSidebar } from '$lib/state/sidebar.svelte';

	interface Props {
		mod: Mod;
		profileMod: ProfileMod;
		profileId: string;
		ondelete?: () => void;
	}

	let { mod, profileMod, profileId, ondelete }: Props = $props();

	const sidebar = getSidebar();
	const contentId = $derived(`profile-${profileId}-mod-${mod.id}`);
	const isSelected = $derived(sidebar.contentId === contentId);

	function openModDetails() {
		sidebar.open(sidebarContent, undefined, contentId);
	}

	function handleDelete(e: MouseEvent) {
		e.stopPropagation();
		ondelete?.();
	}
</script>

{#snippet sidebarContent()}
	<ModDetailsSidebar modId={mod.id} {profileId} onclose={() => sidebar.close()} />
{/snippet}

<Card.Root
	class="overflow-hidden p-0 transition-all hover:bg-accent/50 {isSelected
		? 'border-primary ring-1 ring-primary'
		: 'hover:border-primary/50'}"
>
	<button
		type="button"
		onclick={openModDetails}
		class="relative flex w-full cursor-pointer gap-3 p-3 text-left"
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

		<div class="min-w-0 flex-1 pr-12">
			<div class="mb-0.5 flex items-center gap-2">
				<h3 class="truncate text-base font-bold" title={mod.name}>
					{mod.name}
				</h3>
				<Badge variant="secondary" class="shrink-0 text-[10px]">v{profileMod.version}</Badge>
			</div>
			<p class="mb-1 truncate text-sm text-muted-foreground/80">
				by {mod.author}
			</p>
			<p class="line-clamp-2 text-sm text-muted-foreground">
				{mod.description}
			</p>
		</div>

		<Button
			size="sm"
			variant="destructive"
			class="absolute top-3 right-3 flex-none"
			onclick={handleDelete}
		>
			<Trash class="size-4" />
		</Button>
	</button>
</Card.Root>
