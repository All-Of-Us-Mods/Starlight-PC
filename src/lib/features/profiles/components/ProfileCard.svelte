<script lang="ts">
	import * as Card from '$lib/components/ui/card';
	import { Badge } from '$lib/components/ui/badge';
	import { Button } from '$lib/components/ui/button';
	import { createQuery } from '@tanstack/svelte-query';
	import { modQueries } from '$lib/features/mods/queries';
	import type { Profile, UnifiedMod } from '../schema';
	import type { Mod } from '$lib/features/mods/schema';
	import { gameState } from '../game-state-service.svelte';
	import { profileService } from '../profile-service';
	import { installProgress } from '../install-progress.svelte';
	import { useDeleteUnifiedMod, useRetryBepInExInstall } from '../mutations';
	import { showError } from '$lib/utils/toast';
	import { formatPlayTime } from '$lib/utils';
	import { join } from '@tauri-apps/api/path';
	import { revealItemInDir } from '@tauri-apps/plugin-opener';
	import ProfileCardActions from './ProfileCardActions.svelte';
	import ProfileModsList from './ProfileModsList.svelte';
	import { Package, CircleAlert } from '@lucide/svelte';
	import { CalendarDays, Clock, RotateCcw, Download } from '@jis3r/icons';

	let {
		profile,
		onlaunch,
		ondelete
	}: { profile: Profile; onlaunch?: () => void; ondelete?: () => void } = $props();

	const deleteMod = useDeleteUnifiedMod();
	const retryBepInExInstall = useRetryBepInExInstall();

	let showAllMods = $state(false);

	async function handleOpenFolder() {
		try {
			const fullPath = await join(profile.path, 'BepInEx');
			await revealItemInDir(fullPath);
		} catch (error) {
			showError(error, 'Open folder');
		}
	}

	async function handleRemoveMod(mod: { id: string; source: 'managed' | 'custom' }) {
		try {
			const unifiedMod = unifiedModsQuery.data?.find((m: UnifiedMod) =>
				m.source === 'managed' ? m.mod_id === mod.id : m.file === mod.id
			);
			if (unifiedMod) {
				await deleteMod.mutateAsync({ profileId: profile.id, mod: unifiedMod });
			}
		} catch (error) {
			showError(error, 'Remove mod');
		}
	}

	const lastLaunched = $derived(
		profile.last_launched_at ? new Date(profile.last_launched_at).toLocaleDateString() : 'Never'
	);

	const isRunning = $derived(gameState.isProfileRunning(profile.id));
	const installState = $derived(installProgress.getState(profile.id));
	const isInstalling = $derived(
		profile.bepinex_installed === false || installState?.status === 'installing'
	);
	const hasInstallError = $derived(installState?.status === 'error');
	const isDisabled = $derived(isInstalling || isRunning);

	async function handleRetryInstall() {
		installProgress.clearProgress(profile.id);
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

			<ProfileCardActions
				onlaunch={() => onlaunch?.()}
				onOpenFolder={handleOpenFolder}
				ondelete={() => ondelete?.()}
				allMods={allMods()}
				onRemoveMod={handleRemoveMod}
				{isDisabled}
			/>
		</Card.Header>

		<Card.Content class="pt-4">
			<ProfileModsList
				allMods={allMods()}
				showAllMods
				onToggleShowAll={() => (showAllMods = !showAllMods)}
			/>
		</Card.Content>
	</Card.Root>
</div>

<style lang="postcss">
	@reference "$lib/../app.css";
</style>
