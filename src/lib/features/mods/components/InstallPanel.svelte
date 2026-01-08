<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { Badge } from '$lib/components/ui/badge';
	import { Skeleton } from '$lib/components/ui/skeleton';
	import * as Select from '$lib/components/ui/select';
	import { Switch } from '$lib/components/ui/switch';
	import { Download, Check, TriangleAlert } from '@jis3r/icons';
	import { LoaderCircle, Package } from '@lucide/svelte';
	import { createQuery, useQueryClient } from '@tanstack/svelte-query';
	import { modQueries } from '../queries';
	import { profileQueries } from '$lib/features/profiles/queries';
	import { modInstallService } from '$lib/features/profiles/mod-install-service';
	import { profileService } from '$lib/features/profiles/profile-service';
	import { modDownloadProgress } from '$lib/features/profiles/mod-download-progress.svelte';
	import { showSuccess } from '$lib/utils/toast';
	import type { ModDependency } from '../schema';
	import type { Profile } from '$lib/features/profiles/schema';
	import { onDestroy } from 'svelte';
	import type { UnlistenFn } from '@tauri-apps/api/event';

	interface Props {
		modId: string;
		selectedVersion: string;
		dependencies: ModDependency[];
		onInstallComplete?: () => void;
		onCancel?: () => void;
	}

	let { modId, selectedVersion, dependencies, onInstallComplete, onCancel }: Props = $props();

	const queryClient = useQueryClient();

	// ============ QUERIES ============

	const profilesQuery = createQuery(() => profileQueries.all());
	const profiles = $derived((profilesQuery.data ?? []) as Profile[]);
	const hasProfiles = $derived(profiles.length > 0);

	const resolvedDepsQuery = createQuery(() => modQueries.resolvedDependencies(dependencies));
	const resolvedDeps = $derived(resolvedDepsQuery.data ?? []);
	const installableDependencies = $derived(resolvedDeps.filter((d) => d.type !== 'conflict'));

	// ============ STATE ============

	let selectedProfileId = $state('');
	let isInstalling = $state(false);
	let installError = $state('');
	let selectedDependencies = $state<Set<string>>(new Set());
	let modsBeingInstalled = $state<string[]>([]);
	let progressUnlisten: UnlistenFn | null = null;

	// ============ DERIVED ============

	const selectedProfile = $derived(profiles.find((p) => p.id === selectedProfileId));

	const isModInstalledInProfile = $derived(
		selectedProfile?.mods.some((m) => m.mod_id === modId) ?? false
	);

	const installedDepsInProfile = $derived(
		new Set(selectedProfile?.mods.map((m) => m.mod_id) ?? [])
	);

	const conflictsInProfile = $derived(
		resolvedDeps
			.filter((d) => d.type === 'conflict')
			.filter((d) => installedDepsInProfile.has(d.mod_id))
	);

	// ============ EFFECTS ============

	// Set default profile when profiles load
	$effect(() => {
		if (profiles.length > 0 && !selectedProfileId) {
			const mostRecent = [...profiles].sort((a, b) => b.created_at - a.created_at)[0];
			selectedProfileId = mostRecent.id;
		}
	});

	// Initialize selected dependencies when resolved deps change
	$effect(() => {
		if (resolvedDeps.length > 0) {
			selectedDependencies = new Set(
				resolvedDeps.filter((d) => d.type !== 'conflict').map((d) => d.mod_id)
			);
		}
	});

	// ============ HANDLERS ============

	function toggleDependency(depModId: string) {
		selectedDependencies = new Set(
			selectedDependencies.has(depModId)
				? [...selectedDependencies].filter((id) => id !== depModId)
				: [...selectedDependencies, depModId]
		);
	}

	async function handleInstall() {
		if (!selectedProfile || !selectedVersion) return;

		try {
			isInstalling = true;
			installError = '';

			const modsToInstall = [{ modId, version: selectedVersion }];

			// Add selected dependencies that aren't already installed
			for (const dep of installableDependencies) {
				if (selectedDependencies.has(dep.mod_id) && !installedDepsInProfile.has(dep.mod_id)) {
					modsToInstall.push({ modId: dep.mod_id, version: dep.resolvedVersion });
				}
			}

			modsBeingInstalled = modsToInstall.map((m) => m.modId);

			progressUnlisten = await modInstallService.onDownloadProgress((progress) => {
				modDownloadProgress.setProgress(progress.mod_id, progress);
			});

			const results = await modInstallService.installModsToProfile(
				modsToInstall,
				selectedProfile.path
			);

			for (const result of results) {
				await profileService.addModToProfile(
					selectedProfileId,
					result.modId,
					result.version,
					result.fileName
				);
			}

			// Invalidate profiles query to update installed status
			await queryClient.invalidateQueries({ queryKey: ['profiles'] });

			showSuccess(`Installed to ${selectedProfile.name}`);
			onInstallComplete?.();
		} catch (e) {
			installError = e instanceof Error ? e.message : 'Failed to install';
		} finally {
			isInstalling = false;
			if (progressUnlisten) {
				progressUnlisten();
				progressUnlisten = null;
			}
			for (const id of modsBeingInstalled) {
				modDownloadProgress.clear(id);
			}
			modsBeingInstalled = [];
		}
	}

	// Cleanup listener on component destroy
	onDestroy(() => {
		if (progressUnlisten) {
			progressUnlisten();
			progressUnlisten = null;
		}
	});
</script>

{#if !hasProfiles}
	<!-- No profiles -->
	<div class="p-4">
		<Button disabled class="w-full opacity-50">
			<Package class="mr-2 h-4 w-4" />
			No profiles available
		</Button>
	</div>
{:else}
	<div class="max-h-[50vh] overflow-y-auto border-t border-border/30">
		<div class="space-y-4 p-4">
			<!-- Profile Selector -->
			<div class="space-y-2">
				<span class="text-xs font-medium text-muted-foreground">Install to Profile</span>
				<Select.Root bind:value={selectedProfileId} type="single" disabled={isInstalling}>
					<Select.Trigger class="w-full">
						<span class="flex items-center gap-2">
							<Package class="h-4 w-4 text-muted-foreground" />
							{selectedProfile?.name ?? 'Select profile'}
						</span>
					</Select.Trigger>
					<Select.Content>
						{#each profiles as p (p.id)}
							<Select.Item value={p.id}>
								<span class="flex w-full items-center justify-between gap-3">
									<span>{p.name}</span>
									{#if p.mods.some((m) => m.mod_id === modId)}
										<Badge variant="secondary" class="text-[10px]">Installed</Badge>
									{/if}
								</span>
							</Select.Item>
						{/each}
					</Select.Content>
				</Select.Root>
			</div>

			<!-- Already installed warning -->
			{#if isModInstalledInProfile}
				<div
					class="flex items-center gap-2 rounded-lg bg-amber-500/10 px-3 py-2 text-sm text-amber-600 dark:text-amber-400"
				>
					<TriangleAlert size={16} class="shrink-0" />
					<span>This mod is already installed in this profile</span>
				</div>
			{/if}

			<!-- Dependencies -->
			{#if resolvedDepsQuery.isPending && dependencies.length > 0}
				<div class="space-y-2">
					<span class="text-xs font-medium text-muted-foreground">Dependencies</span>
					<div class="space-y-1.5">
						<Skeleton class="h-10 w-full rounded-lg" />
					</div>
				</div>
			{:else if installableDependencies.length > 0}
				<div class="space-y-2">
					<span class="text-xs font-medium text-muted-foreground">Dependencies</span>
					<div class="space-y-1.5 rounded-lg border border-border/50 p-2">
						{#each installableDependencies as dep (dep.mod_id)}
							{@const isInstalled = selectedProfile?.mods.some((m) => m.mod_id === dep.mod_id)}
							<div
								class="flex items-center justify-between rounded-md px-2 py-1.5 transition-colors hover:bg-muted/50"
							>
								<div class="flex items-center gap-2.5">
									<Switch
										checked={selectedDependencies.has(dep.mod_id)}
										onCheckedChange={() => toggleDependency(dep.mod_id)}
										disabled={isInstalling || isInstalled}
										class="scale-90"
									/>
									<div class="flex flex-col">
										<span class="text-sm leading-tight font-medium">{dep.modName}</span>
										<span class="text-[11px] text-muted-foreground">v{dep.resolvedVersion}</span>
									</div>
								</div>
								<div class="flex items-center gap-1.5">
									{#if isInstalled}
										<Badge variant="secondary" class="text-[10px]">Installed</Badge>
									{:else}
										<Badge
											variant={dep.type === 'required' ? 'default' : 'secondary'}
											class="text-[10px]"
										>
											{dep.type}
										</Badge>
									{/if}
								</div>
							</div>
						{/each}
					</div>
				</div>
			{/if}

			<!-- Conflicts Warning -->
			{#if conflictsInProfile.length > 0}
				<div class="rounded-lg border border-destructive/30 bg-destructive/10 p-3">
					<div class="flex items-start gap-2">
						<TriangleAlert size={16} class="mt-0.5 shrink-0 text-destructive" />
						<div class="space-y-1">
							<p class="text-sm font-medium text-destructive">Conflicts Detected</p>
							<p class="text-xs text-destructive/80">
								This mod conflicts with mods in this profile:
							</p>
							<ul class="list-inside list-disc text-xs text-destructive/80">
								{#each conflictsInProfile as conflict (conflict.mod_id)}
									<li>{conflict.modName}</li>
								{/each}
							</ul>
						</div>
					</div>
				</div>
			{/if}

			<!-- Download Progress -->
			{#if isInstalling && modsBeingInstalled.length > 0}
				<div class="space-y-2">
					<span class="text-xs font-medium text-muted-foreground">Installing...</span>
					<div class="space-y-2 rounded-lg border border-border/50 p-2">
						{#each modsBeingInstalled as downloadingModId (downloadingModId)}
							{@const state = modDownloadProgress.getState(downloadingModId)}
							<div class="flex items-center gap-2 px-1">
								{#if state?.status === 'downloading'}
									<LoaderCircle class="h-3.5 w-3.5 animate-spin text-primary" />
									<div class="min-w-0 flex-1">
										<div class="flex items-center justify-between text-xs">
											<span class="truncate font-medium">{downloadingModId}</span>
											<span class="text-muted-foreground">
												{modDownloadProgress.getStageText(state.progress.stage)}
											</span>
										</div>
										{#if state.progress.stage === 'downloading'}
											<div class="mt-1 h-1 w-full overflow-hidden rounded-full bg-muted">
												<div
													class="h-full bg-primary transition-all duration-150"
													style="width: {state.progress.progress}%"
												></div>
											</div>
										{/if}
									</div>
								{:else if state?.status === 'complete'}
									<Check class="h-3.5 w-3.5 text-green-500" />
									<span class="text-xs font-medium">{downloadingModId}</span>
								{:else}
									<LoaderCircle class="h-3.5 w-3.5 animate-spin text-muted-foreground" />
									<span class="text-xs text-muted-foreground">{downloadingModId}</span>
								{/if}
							</div>
						{/each}
					</div>
				</div>
			{/if}

			<!-- Error -->
			{#if installError}
				<p class="text-sm text-destructive">{installError}</p>
			{/if}
		</div>
	</div>

	<!-- Action buttons -->
	<div class="flex gap-2 p-4">
		<Button variant="outline" class="flex-1" onclick={onCancel} disabled={isInstalling}>
			Cancel
		</Button>
		<Button
			class="flex-1 gap-2"
			onclick={handleInstall}
			disabled={isInstalling || !selectedProfileId || !selectedVersion || isModInstalledInProfile}
		>
			{#if isInstalling}
				<LoaderCircle class="h-4 w-4 animate-spin" />
				Installing...
			{:else}
				<Download size={16} />
				Install
			{/if}
		</Button>
	</div>
{/if}
