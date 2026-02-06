<script lang="ts">
	import { goto } from '$app/navigation';
	import * as Card from '$lib/components/ui/card';
	import { Badge } from '$lib/components/ui/badge';
	import { Button } from '$lib/components/ui/button';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
	import { createMutation, createQuery, useQueryClient } from '@tanstack/svelte-query';
	import { modQueries } from '$lib/features/mods/queries';
	import type { Profile, UnifiedMod } from '../schema';
	import type { Mod } from '$lib/features/mods/schema';
	import { gameState } from '../game-state.svelte';
	import { profileMutations } from '../mutations';
	import { showError } from '$lib/utils/toast';
	import { formatPlayTime } from '$lib/utils';
	import { join } from '@tauri-apps/api/path';
	import { revealItemInDir } from '@tauri-apps/plugin-opener';
	import ModDetailsSidebar from '$lib/features/mods/components/ModDetailsSidebar.svelte';
	import { getSidebar } from '$lib/state/sidebar.svelte';
	import { Package, CircleAlert, Play, FolderOpen, EllipsisVertical } from '@lucide/svelte';
	import { CalendarDays, Clock, RotateCcw, Download, Trash2 } from '@jis3r/icons';
	import { profileQueries } from '../queries';
	import { watchDirectory } from '$lib/utils/file-watcher';
	import { info } from '@tauri-apps/plugin-log';

	let {
		profile,
		onlaunch,
		ondelete
	}: { profile: Profile; onlaunch?: () => void; ondelete?: () => void } = $props();

	const queryClient = useQueryClient();
	const sidebar = getSidebar();
	let selectedModId = $state<string | null>(null);

	const deleteMod = createMutation(() => profileMutations.deleteUnifiedMod(queryClient));
	const cleanupMods = createMutation(() => profileMutations.cleanupMissingMods(queryClient));
	const retryBepInExInstall = createMutation(() =>
		profileMutations.retryBepInExInstall(queryClient)
	);

	let unwatchFn: (() => void) | null = null;

	async function handleRemoveMod(mod: { id: string; source: 'managed' | 'custom' }) {
		try {
			const unifiedMod = unifiedMods().find((m) =>
				m.source === 'managed' ? m.mod_id === mod.id : m.file === mod.id
			);
			if (unifiedMod) {
				await deleteMod.mutateAsync({ profileId: profile.id, mod: unifiedMod });
			}
		} catch (error) {
			showError(error, 'Remove mod');
		}
	}

	async function setupModsWatcher(): Promise<(() => void) | undefined> {
		if (!profile.bepinex_installed) return;

		try {
			const pluginsPath = await join(profile.path, 'BepInEx', 'plugins');
			info(`Setting up mods watcher for: ${pluginsPath}`);

			return await watchDirectory(pluginsPath, async () => {
				queryClient.invalidateQueries({ queryKey: ['disk-files', profile.path] });
				try {
					await cleanupMods.mutateAsync(profile.id);
					info(`Cleaned up mods for profile: ${profile.id}`);
				} catch (err) {
					info(`Failed to cleanup mods: ${err}`);
				}
			});
		} catch (err) {
			info(`Could not setup mods watcher: ${err}`);
			return undefined;
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

	let showAllMods = $state(false);

	async function handleOpenFolder() {
		try {
			const fullPath = await join(profile.path, 'BepInEx');
			await revealItemInDir(fullPath);
		} catch (error) {
			showError(error, 'Open folder');
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
		new Map(
			modsQueries
				.map((q) => q.data)
				.filter((m): m is Mod => m !== undefined)
				.map((m) => [m.id, m])
		)
	);

	const diskFilesQuery = createQuery(() => profileQueries.diskFiles(profile.path));

	const unifiedMods = $derived(() => {
		const diskFiles = diskFilesQuery.data ?? [];
		const managedFiles = new Set(profile.mods.map((m) => m.file).filter(Boolean));

		const unified: UnifiedMod[] = profile.mods
			.filter((m) => m.file && diskFiles.includes(m.file))
			.map((mod) => ({
				source: 'managed' as const,
				mod_id: mod.mod_id,
				version: mod.version,
				file: mod.file!
			}));

		for (const file of diskFiles) {
			if (!managedFiles.has(file)) {
				unified.push({ source: 'custom' as const, file });
			}
		}

		return unified;
	});

	const allMods = $derived(() => {
		const unified = unifiedMods();
		return unified.map((mod) => {
			if (mod.source === 'managed') {
				const modInfo = modsMap.get(mod.mod_id);
				return { id: mod.mod_id, name: modInfo?.name ?? mod.mod_id, source: 'managed' as const };
			}
			return { id: mod.file, name: mod.file, source: 'custom' as const };
		});
	});

	const displayedMods = $derived(() => (showAllMods ? allMods() : allMods().slice(0, 3)));
	const hiddenModCount = $derived(() => allMods().length - 3);

	const modCount = $derived(unifiedMods().length);

	// Setup file watcher for mods directory
	$effect(() => {
		if (profile.bepinex_installed) {
			let mounted = true;

			const effectCleanup = async () => {
				const cleanup = await setupModsWatcher();
				if (mounted && cleanup) {
					unwatchFn = cleanup;
				} else if (cleanup) {
					cleanup();
				}
			};
			effectCleanup();

			return () => {
				mounted = false;
				if (unwatchFn) {
					unwatchFn();
					unwatchFn = null;
				}
			};
		}
	});
</script>

<div class="@container">
	<Card.Root
		class="cursor-pointer transition-all hover:bg-accent/50 {isRunning
			? 'bg-green-500/5 ring-2 ring-green-500/50'
			: ''}"
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
							onclick={handleRetryInstall}
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
				<Button size="sm" onclick={() => onlaunch?.()} disabled={isDisabled}>
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
							<DropdownMenu.Item onclick={() => onlaunch?.()} disabled={isDisabled}>
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
								<DropdownMenu.SubTrigger disabled={isDisabled}>
									<Package class="size-4" />
									Manage Mods
								</DropdownMenu.SubTrigger>
								<DropdownMenu.SubContent class="max-h-64 overflow-y-auto">
									{#each allMods() as mod (mod.id)}
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
			{#if allMods().length > 0}
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
