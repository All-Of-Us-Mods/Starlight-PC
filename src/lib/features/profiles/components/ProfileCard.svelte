<script lang="ts">
	import { goto } from '$app/navigation';
	import { onDestroy } from 'svelte';
	import * as Card from '$lib/components/ui/card';
	import { Badge } from '$lib/components/ui/badge';
	import { Button } from '$lib/components/ui/button';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
	import { createMutation, createQuery, useQueryClient } from '@tanstack/svelte-query';
	import { save as saveDialog } from '@tauri-apps/plugin-dialog';
	import { modQueries } from '$lib/features/mods/queries';
	import { mapModsById } from '$lib/features/mods/ui/mod-query-controller';
	import type { Profile } from '../schema';
	import type { Mod } from '$lib/features/mods/schema';
	import { gameState } from '../game-state.svelte';
	import { profileMutations } from '../mutations';
	import { showError, showSuccess } from '$lib/utils/toast';
	import { formatPlayTime } from '$lib/utils';
	import ModDetailsSidebar from '$lib/features/mods/components/ModDetailsSidebar.svelte';
	import { getSidebar } from '$lib/state/sidebar.svelte';
	import { Package, CircleAlert, Play, FolderOpen, EllipsisVertical } from '@lucide/svelte';
	import { CalendarDays, Clock, RotateCcw, Download, Trash2 } from '@jis3r/icons';
	import { profileQueries } from '../queries';
	import {
		buildProfileModChips,
		buildUnifiedMods,
		findUnifiedModByChip,
		openProfileFolder
	} from '$lib/features/profiles/ui/profile-card-controller';

	let {
		profile,
		onlaunch,
		ondelete,
		allowMultiInstanceLaunch = false
	}: {
		profile: Profile;
		onlaunch?: () => void;
		ondelete?: () => void;
		allowMultiInstanceLaunch?: boolean;
	} = $props();

	const queryClient = useQueryClient();
	const sidebar = getSidebar();
	let selectedModId = $state<string | null>(null);

	const deleteMod = createMutation(() => profileMutations.deleteUnifiedMod(queryClient));
	const retryBepInExInstall = createMutation(() =>
		profileMutations.retryBepInExInstall(queryClient)
	);
	const exportZip = createMutation(() => profileMutations.exportZip());

	async function handleRemoveMod(mod: { id: string; source: 'managed' | 'custom' }) {
		try {
			const unifiedMod = findUnifiedModByChip(mod.id, mod.source, unifiedMods());
			if (unifiedMod) {
				await deleteMod.mutateAsync({ profileId: profile.id, mod: unifiedMod });
			}
		} catch (error) {
			showError(error, 'Remove mod');
		}
	}

	function handleModClick(modId: string) {
		const contentId = `profile-${profile.id}-mod-${modId}`;
		const opened = sidebar.open(sidebarContent, () => (selectedModId = null), contentId);
		selectedModId = opened ? modId : null;
	}

	function closeSidebar() {
		selectedModId = null;
		sidebar.close();
	}

	onDestroy(() => {
		if (!selectedModId) return;
		const contentId = `profile-${profile.id}-mod-${selectedModId}`;
		if (sidebar.contentId === contentId) {
			sidebar.close();
			sidebar.finalizeClose();
		}
	});

	let showAllMods = $state(false);

	async function handleOpenFolder() {
		await openProfileFolder(profile.path);
	}

	async function handleExportProfile() {
		try {
			const destination = await saveDialog({
				title: 'Export Profile ZIP',
				defaultPath: `${profile.name}.zip`,
				filters: [{ name: 'ZIP Archive', extensions: ['zip'] }]
			});
			if (!destination) return;
			await exportZip.mutateAsync({ profileId: profile.id, destination });
			showSuccess(`Exported "${profile.name}"`);
		} catch (error) {
			showError(error, 'Export profile');
		}
	}

	const lastLaunched = $derived(
		profile.last_launched_at ? new Date(profile.last_launched_at).toLocaleDateString() : 'Never'
	);

	const isRunning = $derived(gameState.isProfileRunning(profile.id));
	const installState = $derived(gameState.getBepInExState(profile.id));
	const isInstalling = $derived(
		profile.bepinex_installed === false || installState?.status === 'installing'
	);
	const hasInstallError = $derived(installState?.status === 'error');
	const isDisabled = $derived(isInstalling || isRunning);
	const isLaunchDisabled = $derived(isInstalling || (isRunning && !allowMultiInstanceLaunch));
	const launchLabel = $derived(
		isRunning ? (allowMultiInstanceLaunch ? 'Launch Another' : 'Running') : 'Launch'
	);

	async function handleRetryInstall() {
		gameState.clearBepInExProgress(profile.id);
		await retryBepInExInstall.mutateAsync({ profileId: profile.id, profilePath: profile.path });
	}

	const totalPlayTime = $derived(
		(profile.total_play_time ?? 0) + (isRunning ? gameState.getSessionDuration() : 0)
	);

	const modIds = $derived(profile.mods.map((m) => m.mod_id));
	const modsQueries = $derived(modIds.map((id) => createQuery(() => modQueries.byId(id))));
	const modsMap = $derived(
		mapModsById(modsQueries.map((query) => query.data) as Array<Mod | undefined>)
	);

	const diskFilesQuery = createQuery(() => profileQueries.diskFiles(profile.path));

	const unifiedMods = $derived(() => {
		const diskFiles = diskFilesQuery.data ?? [];
		return buildUnifiedMods(profile, diskFiles);
	});

	const allMods = $derived(buildProfileModChips(unifiedMods(), modsMap));

	const displayedMods = $derived(() => (showAllMods ? allMods : allMods.slice(0, 3)));
	const hiddenModCount = $derived(() => allMods.length - 3);

	const modCount = $derived(unifiedMods().length);
</script>

<div class="@container">
	<Card.Root
		class="cursor-pointer transition-all hover:bg-accent/50 {isRunning
			? 'bg-green-500/5 ring-2 ring-green-500/50'
			: 'hover:border-primary/50'}"
		onclick={() => goto(`/library/${profile.id}`)}
	>
		<Card.Header class="gap-4 @md:flex-row @md:items-start @md:justify-between">
			<div class="min-w-0 flex-1 space-y-1.5">
				<div class="flex flex-wrap items-center gap-2">
					<Card.Title class="truncate" title={profile.name}>
						{profile.name}
					</Card.Title>
					{#if hasInstallError}
						<Badge variant="outline" class="gap-1.5 border-destructive/50 text-destructive">
							<CircleAlert class="size-3" />
							Install failed
						</Badge>
						<Button
							variant="ghost"
							size="icon"
							class="size-6"
							onclick={(e) => {
								e.stopPropagation();
								handleRetryInstall();
							}}
							title="Retry installation"
						>
							<RotateCcw class="size-3" />
						</Button>
					{:else if isInstalling}
						<Badge
							variant="outline"
							class="gap-1.5 border-amber-500/50 text-amber-600 dark:text-amber-400"
						>
							<Download class="animate-pulse" size={12} />
							{installState?.status === 'installing'
								? installState.progress.message
								: 'Installing...'}
						</Badge>
					{/if}
				</div>
				<Card.Description class="flex flex-wrap items-center gap-x-3 gap-y-1">
					<span class="inline-flex items-center gap-1.5">
						<Package class="size-3.5" />
						{modCount} mod{modCount !== 1 ? 's' : ''}
					</span>
					<span class="inline-flex items-center gap-1.5">
						<CalendarDays size={14} />
						{lastLaunched}
					</span>
					<span class="inline-flex items-center gap-1.5">
						<Clock size={14} />
						{formatPlayTime(totalPlayTime)}
					</span>
				</Card.Description>
			</div>

			<!-- Actions -->
			<div class="flex items-center gap-2 @md:shrink-0">
				<Button
					size="sm"
					onclick={(e) => {
						e.stopPropagation();
						onlaunch?.();
					}}
					disabled={isLaunchDisabled}
				>
					<Play class="size-4 fill-current" />
					<span>{launchLabel}</span>
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
							<DropdownMenu.Item onclick={() => onlaunch?.()} disabled={isLaunchDisabled}>
								<Play class="size-4" />
								{launchLabel}
							</DropdownMenu.Item>
							<DropdownMenu.Item onclick={handleOpenFolder}>
								<FolderOpen class="size-4" />
								Open Folder
							</DropdownMenu.Item>
							<DropdownMenu.Item onclick={handleExportProfile}>
								<Download class="size-4" />
								Export ZIP
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
											onclick={() => handleRemoveMod(mod)}
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
							onclick={() => ondelete?.()}
							class="text-destructive focus:bg-destructive focus:text-destructive-foreground"
							disabled={isDisabled}
						>
							<Trash2 class="size-4" />
							Delete Profile
						</DropdownMenu.Item>
					</DropdownMenu.Content>
				</DropdownMenu.Root>
			</div>
		</Card.Header>

		<Card.Content class="pt-4">
			<!-- Mods List -->
			{#if allMods.length > 0}
				<div class="flex flex-wrap items-center gap-1.5">
					{#each displayedMods() as mod (mod.id)}
						{#if mod.source === 'managed'}
							<button
								type="button"
								onclick={(e) => {
									e.stopPropagation();
									handleModClick(mod.id);
								}}
								class="inline-flex max-w-32 items-center truncate rounded-md border border-transparent bg-secondary px-2.5 py-0.5 text-xs font-semibold text-secondary-foreground transition-colors hover:bg-secondary/80 hover:text-primary"
							>
								{mod.name}
							</button>
						{:else}
							<Badge variant="secondary" class="max-w-32 truncate text-xs">
								{mod.name}
							</Badge>
						{/if}
					{/each}
					{#if hiddenModCount() > 0}
						<button
							type="button"
							onclick={() => (showAllMods = !showAllMods)}
							class="rounded-md px-2 py-0.5 text-xs text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
						>
							{showAllMods ? 'Show less' : `+${hiddenModCount()} more`}
						</button>
					{/if}
				</div>
			{/if}
		</Card.Content>
	</Card.Root>
</div>

{#snippet sidebarContent()}
	{#if selectedModId}
		<ModDetailsSidebar modId={selectedModId} profileId={profile.id} onclose={closeSidebar} />
	{/if}
{/snippet}
