<script lang="ts">
	import * as Card from '$lib/components/ui/card';
	import { Button } from '$lib/components/ui/button';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
	import * as Dialog from '$lib/components/ui/dialog';
	import { Badge } from '$lib/components/ui/badge';
	import {
		Play,
		FolderOpen,
		Trash2,
		Calendar,
		Package,
		EllipsisVertical,
		Download,
		Clock,
		AlertTriangle
	} from '@lucide/svelte';
	import { revealItemInDir } from '@tauri-apps/plugin-opener';
	import { createQuery } from '@tanstack/svelte-query';
	import { modQueries } from '$lib/features/mods/queries';
	import type { Profile, ProfileMod } from '../schema';
	import type { Mod } from '$lib/features/mods/schema';
	import { join } from '@tauri-apps/api/path';

	let {
		profile,
		onlaunch,
		ondelete,
		onremove
	}: {
		profile: Profile;
		onlaunch?: () => void;
		ondelete?: () => void;
		onremove?: (mod: ProfileMod) => void;
	} = $props();

	let showAllMods = $state(false);
	let removeModDialogOpen = $state(false);
	let modToRemove = $state<{ mod: ProfileMod; modInfo?: Mod } | null>(null);

	async function handleOpenFolder() {
		try {
			const fullPath = await join(profile.path, 'BepInEx');
			await revealItemInDir(fullPath);
		} catch (error) {
			console.error('Failed to open folder:', error);
		}
	}

	async function handleRemoveMod(mod: ProfileMod) {
		const modInfo = modsMap.get(mod.mod_id);
		modToRemove = { mod, modInfo };
		removeModDialogOpen = true;
	}

	function confirmRemoveMod() {
		if (modToRemove) {
			onremove?.(modToRemove.mod);
			modToRemove = null;
			removeModDialogOpen = false;
		}
	}

	const lastLaunched = $derived(
		profile.last_launched_at ? new Date(profile.last_launched_at).toLocaleDateString() : 'Never'
	);

	const isRecentlyLaunched = $derived(
		profile.last_launched_at !== undefined && Date.now() - profile.last_launched_at < 5 * 60 * 1000
	);

	const cardClass = $derived(isRecentlyLaunched ? 'ring-2 ring-primary bg-primary/5' : '');

	const modIds = $derived(profile.mods.map((m) => m.mod_id));
	const modsQueries = $derived(modIds.map((id) => createQuery(() => modQueries.byId(id))));

	const modsMap = $derived(
		new Map(
			modsQueries
				.map((q) => q.data)
				.filter((m): m is Mod => m !== undefined)
				.map((m) => [m.id, m])
		)
	);

	const displayedMods = $derived(showAllMods ? profile.mods : profile.mods.slice(0, 3));

	const hasMoreMods = $derived(profile.mods.length > 3);
</script>

<Card.Root class="overflow-hidden transition-all hover:bg-accent/50 {cardClass}">
	<Card.Content class="p-4">
		<div class="flex items-start justify-between gap-4">
			<div class="flex min-w-0 flex-1 flex-col gap-2">
				<div class="flex items-center gap-2">
					<h3 class="truncate text-lg font-bold" title={profile.name}>{profile.name}</h3>
					{#if profile.bepinex_installed === false}
						<div
							class="flex items-center gap-1.5 rounded-md bg-amber-500/10 px-2 py-0.5 text-xs font-medium text-amber-700 dark:text-amber-400"
							title="BepInEx is being installed in the background"
						>
							<Download class="h-3 w-3 animate-pulse" />
							<span>Installing...</span>
						</div>
					{/if}
					{#if isRecentlyLaunched}
						<div
							class="flex items-center gap-1 rounded-full bg-primary/10 px-2 py-0.5 text-xs font-medium text-primary"
						>
							<Clock class="h-3 w-3" />
							<span>Just launched</span>
						</div>
					{/if}
				</div>
				<div class="flex items-center gap-4 text-sm text-muted-foreground">
					<div class="flex items-center gap-1.5">
						<Package class="h-4 w-4" />
						<span>{profile.mods.length} mods</span>
					</div>
					<div class="flex items-center gap-1.5">
						<Calendar class="h-4 w-4" />
						<span>{lastLaunched}</span>
					</div>
				</div>

				{#if profile.mods.length > 0}
					<div class="mt-1 flex flex-wrap items-center gap-1.5">
						{#each displayedMods as mod (mod.mod_id)}
							<Badge variant="secondary" class="text-xs">
								{#if modsMap.has(mod.mod_id)}
									{modsMap.get(mod.mod_id)?.name}
								{:else}
									{mod.mod_id}
								{/if}
							</Badge>
						{/each}
						{#if hasMoreMods && !showAllMods}
							<button
								onclick={() => (showAllMods = true)}
								class="text-xs text-muted-foreground hover:text-foreground"
							>
								+{profile.mods.length - 3} more
							</button>
						{:else if showAllMods}
							<button
								onclick={() => (showAllMods = false)}
								class="text-xs text-muted-foreground hover:text-foreground"
							>
								Show less
							</button>
						{/if}
					</div>
				{:else}
					<div class="mt-1 text-xs text-muted-foreground">No mods installed</div>
				{/if}
			</div>

			<div class="flex items-center gap-2">
				<Button
					size="sm"
					onclick={onlaunch}
					disabled={profile.bepinex_installed === false}
					class={isRecentlyLaunched ? 'bg-primary hover:bg-primary/90' : ''}
				>
					<Play class="mr-2 h-4 w-4 fill-current" />
					Launch
				</Button>

				<DropdownMenu.Root>
					<DropdownMenu.Trigger>
						{#snippet child({ props })}
							<Button {...props} variant="ghost" size="icon" aria-label="Profile actions">
								<EllipsisVertical class="h-5 w-5" />
							</Button>
						{/snippet}
					</DropdownMenu.Trigger>
					<DropdownMenu.Content align="end">
						<DropdownMenu.Group>
							<DropdownMenu.Item onclick={onlaunch} disabled={profile.bepinex_installed === false}>
								<Play class="mr-2 h-4 w-4" />
								<span>Launch</span>
							</DropdownMenu.Item>
							<DropdownMenu.Item onclick={handleOpenFolder}>
								<FolderOpen class="mr-2 h-4 w-4" />
								<span>Open Folder</span>
							</DropdownMenu.Item>
						</DropdownMenu.Group>

						{#if profile.mods.length > 0}
							<DropdownMenu.Separator />
							<DropdownMenu.Sub>
								<DropdownMenu.SubTrigger>
									<Package class="mr-2 h-4 w-4" />
									<span>Manage Mods</span>
								</DropdownMenu.SubTrigger>
								<DropdownMenu.SubContent>
									{#each profile.mods as mod (mod.mod_id)}
										<DropdownMenu.Item onclick={() => handleRemoveMod(mod)}>
											{#if modsMap.has(mod.mod_id)}
												{modsMap.get(mod.mod_id)?.name}
											{:else}
												{mod.mod_id}
											{/if}
											<Trash2 class="ml-auto h-4 w-4 text-destructive" />
										</DropdownMenu.Item>
									{/each}
								</DropdownMenu.SubContent>
							</DropdownMenu.Sub>
						{/if}

						<DropdownMenu.Separator />
						<DropdownMenu.Item
							onclick={ondelete}
							class="text-destructive focus:bg-destructive focus:text-destructive-foreground"
						>
							<Trash2 class="mr-2 h-4 w-4" />
							<span>Delete</span>
						</DropdownMenu.Item>
					</DropdownMenu.Content>
				</DropdownMenu.Root>
			</div>
		</div>
	</Card.Content>
</Card.Root>

<Dialog.Root bind:open={removeModDialogOpen}>
	<Dialog.Content>
		<Dialog.Header>
			<Dialog.Title>Remove Mod from Profile?</Dialog.Title>
			<Dialog.Description>
				{#if modToRemove?.modInfo}
					This will remove <strong>{modToRemove.modInfo.name}</strong> from
					<strong>{profile.name}</strong>.
				{:else}
					This will remove a mod from <strong>{profile.name}</strong>.
				{/if}
			</Dialog.Description>
		</Dialog.Header>

		<div class="flex items-start gap-3 rounded-lg bg-muted p-3 text-sm">
			<AlertTriangle class="mt-0.5 h-4 w-4 shrink-0 text-amber-500" />
			<p class="text-muted-foreground">
				This action will delete mod file from the profile. You can reinstall it later from the
				Explore page.
			</p>
		</div>

		<div class="flex justify-end gap-2">
			<Dialog.Close>
				<Button variant="outline">Cancel</Button>
			</Dialog.Close>
			<Button variant="destructive" onclick={confirmRemoveMod}>Remove Mod</Button>
		</div>
	</Dialog.Content>
</Dialog.Root>
