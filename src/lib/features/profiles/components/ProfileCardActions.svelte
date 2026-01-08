<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
	import { Play, FolderOpen, Package, EllipsisVertical } from '@lucide/svelte';
	import { Trash2 } from '@jis3r/icons';

	let {
		onlaunch,
		onOpenFolder,
		ondelete,
		allMods,
		onRemoveMod,
		isDisabled
	}: {
		onlaunch: () => void;
		onOpenFolder: () => void;
		ondelete: () => void;
		allMods: Array<{ id: string; name: string; source: 'managed' | 'custom' }>;
		onRemoveMod: (mod: { id: string; source: 'managed' | 'custom' }) => void;
		isDisabled: boolean;
	} = $props();
</script>

<div class="flex items-center gap-2 @md:shrink-0">
	<Button size="sm" onclick={onlaunch} disabled={isDisabled}>
		<Play class="size-4 fill-current" />
		<span>Launch</span>
	</Button>

	<DropdownMenu.Root>
		<DropdownMenu.Trigger>
			{#snippet child({ props })}
				<Button {...props} variant="ghost" size="icon" class="size-8">
					<EllipsisVertical class="size-4" />
					<span class="sr-only">Profile actions</span>
				</Button>
			{/snippet}
		</DropdownMenu.Trigger>
		<DropdownMenu.Content align="end" class="w-48">
			<DropdownMenu.Group>
				<DropdownMenu.Item onclick={onlaunch} disabled={isDisabled}>
					<Play class="size-4" />
					Launch
				</DropdownMenu.Item>
				<DropdownMenu.Item onclick={onOpenFolder}>
					<FolderOpen class="size-4" />
					Open Folder
				</DropdownMenu.Item>
			</DropdownMenu.Group>

			{#if allMods.length > 0}
				<DropdownMenu.Separator />
				<DropdownMenu.Sub>
					<DropdownMenu.SubTrigger disabled={isDisabled}>
						<Package class="size-4" />
						Manage Mods
					</DropdownMenu.SubTrigger>
					<DropdownMenu.SubContent class="max-h-64 overflow-y-auto">
						{#each allMods as mod (mod.id)}
							<DropdownMenu.Item
								onclick={() => onRemoveMod(mod)}
								class="justify-between"
								disabled={isDisabled}
							>
								<span class="truncate">{mod.name}</span>
								<Trash2 class="size-4 shrink-0 text-destructive" />
							</DropdownMenu.Item>
						{/each}
					</DropdownMenu.SubContent>
				</DropdownMenu.Sub>
			{/if}

			<DropdownMenu.Separator />
			<DropdownMenu.Item
				onclick={ondelete}
				class="text-destructive focus:bg-destructive focus:text-destructive-foreground"
				disabled={isDisabled}
			>
				<Trash2 class="size-4" />
				Delete Profile
			</DropdownMenu.Item>
		</DropdownMenu.Content>
	</DropdownMenu.Root>
</div>
