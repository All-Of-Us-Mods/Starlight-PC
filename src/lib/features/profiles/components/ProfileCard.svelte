<script lang="ts">
	import * as Card from '$lib/components/ui/card';
	import { Button } from '$lib/components/ui/button';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
	import { Badge } from '$lib/components/ui/badge';
	import {
		Play,
		FolderOpen,
		Trash2,
		Calendar,
		Package,
		EllipsisVertical,
		Download,
		LoaderCircle,
		Clock
	} from '@lucide/svelte';
	import { revealItemInDir } from '@tauri-apps/plugin-opener';
	import { createQuery } from '@tanstack/svelte-query';
	import { modQueries } from '$lib/features/mods/queries';
	import type { Profile, UnifiedMod } from '../schema';
	import type { Mod } from '$lib/features/mods/schema';
	import { join } from '@tauri-apps/api/path';
	import { gameState } from '../game-state-service.svelte';
	import { profileService } from '../profile-service';
	import { queryClient } from '$lib/state/queryClient';

	let {
		profile,
		onlaunch,
		ondelete
	}: { profile: Profile; onlaunch?: () => void; ondelete?: () => void } = $props();

	let showAllMods = $state(false);

	async function handleOpenFolder() {
		try {
			const fullPath = await join(profile.path, 'BepInEx');
			await revealItemInDir(fullPath);
		} catch (error) {
			console.error('Failed to open folder:', error);
		}
	}

	async function handleRemoveMod(mod: { id: string; source: 'managed' | 'custom' }) {
		try {
			const unifiedMod = unifiedModsQuery.data?.find((m: UnifiedMod) =>
				m.source === 'managed' ? m.mod_id === mod.id : m.file === mod.id
			);
			if (unifiedMod) {
				await profileService.deleteUnifiedMod(profile.id, unifiedMod);
			}
			queryClient.invalidateQueries({ queryKey: ['unified-mods', profile.id] });
			queryClient.invalidateQueries({ queryKey: ['profiles'] });
		} catch (error) {
			console.error('Failed to remove mod:', error);
		}
	}

	function formatPlayTime(ms: number): string {
		const seconds = Math.floor(ms / 1000);
		const minutes = Math.floor(seconds / 60);
		const hours = Math.floor(minutes / 60);

		if (hours > 0) {
			const remainingMinutes = minutes % 60;
			return remainingMinutes > 0 ? `${hours}h ${remainingMinutes}m` : `${hours}h`;
		}
		if (minutes > 0) return `${minutes}m`;
		return seconds > 0 ? `${seconds}s` : '0m';
	}

	const lastLaunched = $derived(
		profile.last_launched_at ? new Date(profile.last_launched_at).toLocaleDateString() : 'Never'
	);

	const isRunning = $derived(gameState.isProfileRunning(profile.id));
	const isInstalling = $derived(profile.bepinex_installed === false);
	const isDisabled = $derived(isInstalling || isRunning);

	const totalPlayTime = $derived(
		(profile.total_play_time ?? 0) + (isRunning ? gameState.getSessionDuration() : 0)
	);

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

	const unifiedModsQuery = createQuery(() => ({
		queryKey: ['unified-mods', profile.id],
		queryFn: () => profileService.getUnifiedMods(profile.id),
		staleTime: 1000 * 10
	}));

	const modCount = $derived(unifiedModsQuery.data?.length ?? 0);

	const allMods = $derived(() => {
		const unified = unifiedModsQuery.data ?? [];
		return unified.map((mod) => {
			if (mod.source === 'managed') {
				const modInfo = modsMap.get(mod.mod_id);
				return { id: mod.mod_id, name: modInfo?.name ?? mod.mod_id, source: 'managed' as const };
			}
			return { id: mod.file, name: mod.file, source: 'custom' as const };
		});
	});

	const displayedMods = $derived(showAllMods ? allMods() : allMods().slice(0, 3));
	const hiddenModCount = $derived(allMods().length - 3);
</script>

<div class="@container">
	<Card.Root
		class="transition-all hover:bg-accent/50 {isRunning
			? 'bg-green-500/5 ring-2 ring-green-500/50'
			: ''}"
	>
		<Card.Header class="gap-4 @md:flex-row @md:items-start @md:justify-between">
			<div class="min-w-0 flex-1 space-y-1.5">
				<div class="flex flex-wrap items-center gap-2">
					<Card.Title class="truncate" title={profile.name}>
						{profile.name}
					</Card.Title>
					{#if isInstalling}
						<Badge
							variant="outline"
							class="gap-1.5 border-amber-500/50 text-amber-600 dark:text-amber-400"
						>
							<Download class="size-3 animate-pulse" />
							Installing
						</Badge>
					{/if}
					{#if isRunning}
						<Badge
							variant="outline"
							class="gap-1.5 border-green-500/50 text-green-600 dark:text-green-400"
						>
							<LoaderCircle class="size-3 animate-spin" />
							Running
						</Badge>
					{/if}
				</div>
				<Card.Description class="flex flex-wrap items-center gap-x-3 gap-y-1">
					<span class="inline-flex items-center gap-1.5">
						<Package class="size-3.5" />
						{modCount} mod{modCount !== 1 ? 's' : ''}
					</span>
					<span class="inline-flex items-center gap-1.5">
						<Calendar class="size-3.5" />
						{lastLaunched}
					</span>
					<span class="inline-flex items-center gap-1.5">
						<Clock class="size-3.5" />
						{formatPlayTime(totalPlayTime)}
					</span>
				</Card.Description>
			</div>

			<div class="flex items-center gap-2 @md:shrink-0">
				<Button size="sm" onclick={onlaunch} disabled={isDisabled}>
					{#if isRunning}
						<LoaderCircle class="size-4 animate-spin" />
						<span class="hidden @md:inline">Running</span>
					{:else}
						<Play class="size-4 fill-current" />
						<span class="hidden @md:inline">Launch</span>
					{/if}
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
							<DropdownMenu.Item onclick={handleOpenFolder}>
								<FolderOpen class="size-4" />
								Open Folder
							</DropdownMenu.Item>
						</DropdownMenu.Group>

						{#if allMods().length > 0}
							<DropdownMenu.Separator />
							<DropdownMenu.Sub>
								<DropdownMenu.SubTrigger>
									<Package class="size-4" />
									Manage Mods
								</DropdownMenu.SubTrigger>
								<DropdownMenu.SubContent class="max-h-64 overflow-y-auto">
									{#each allMods() as mod (mod.id)}
										<DropdownMenu.Item onclick={() => handleRemoveMod(mod)} class="justify-between">
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
						>
							<Trash2 class="size-4" />
							Delete Profile
						</DropdownMenu.Item>
					</DropdownMenu.Content>
				</DropdownMenu.Root>
			</div>
		</Card.Header>

		{#if allMods().length > 0}
			<Card.Content class="pt-4">
				<div class="flex flex-wrap items-center gap-1.5">
					{#each displayedMods as mod (mod.id)}
						<Badge variant="secondary" class="max-w-32 truncate text-xs">
							{mod.name}
						</Badge>
					{/each}
					{#if hiddenModCount > 0}
						<button
							type="button"
							onclick={() => (showAllMods = !showAllMods)}
							class="rounded-md px-2 py-0.5 text-xs text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
						>
							{showAllMods ? 'Show less' : `+${hiddenModCount} more`}
						</button>
					{/if}
				</div>
			</Card.Content>
		{/if}
	</Card.Root>
</div>
