<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { Badge } from '$lib/components/ui/badge';
	import { Skeleton } from '$lib/components/ui/skeleton';
	import * as Select from '$lib/components/ui/select';
	import { Switch } from '$lib/components/ui/switch';
	import Prose from '$lib/components/shared/Prose.svelte';
	import { marked } from 'marked';
	import {
		X,
		Maximize,
		Minimize,
		ImageOff,
		Download,
		Clock,
		Check,
		TriangleAlert
	} from '@jis3r/icons';
	import {
		ExternalLink,
		Github,
		Globe,
		MessageCircle,
		ChevronDown,
		ChevronUp,
		Trash2,
		Loader2,
		Package
	} from '@lucide/svelte';
	import { createQuery, useQueryClient } from '@tanstack/svelte-query';
	import { modQueries } from '../queries';
	import { profileQueries } from '$lib/features/profiles/queries';
	import { getSidebar } from '$lib/state/sidebar.svelte';
	import { modInstallService } from '$lib/features/profiles/mod-install-service';
	import { profileService } from '$lib/features/profiles/profile-service';
	import { useDeleteUnifiedMod } from '$lib/features/profiles/mutations';
	import { modDownloadProgress } from '$lib/features/profiles/mod-download-progress.svelte';
	import { showError, showSuccess } from '$lib/utils/toast';
	import type { ModDependency } from '../schema';
	import type { UnifiedMod, Profile } from '$lib/features/profiles/schema';
	import { onDestroy } from 'svelte';
	import type { UnlistenFn } from '@tauri-apps/api/event';

	interface Props {
		modId: string;
		profileId?: string;
		onclose?: () => void;
	}

	let { modId, profileId, onclose }: Props = $props();

	const sidebar = getSidebar();
	const queryClient = useQueryClient();
	const deleteMod = useDeleteUnifiedMod();

	// ============ QUERIES ============

	// Mod data queries
	const modQuery = createQuery(() => modQueries.byId(modId));
	const modInfoQuery = createQuery(() => modQueries.info(modId));
	const versionsQuery = createQuery(() => modQueries.versions(modId));

	// All profiles query
	const profilesQuery = createQuery(() => profileQueries.all());

	// Profile context query - only when opened from a profile (for remove functionality)
	const unifiedModsQuery = createQuery(() => ({
		queryKey: ['unified-mods', profileId] as const,
		queryFn: () => profileService.getUnifiedMods(profileId!),
		enabled: !!profileId,
		staleTime: 1000 * 10
	}));

	// ============ STATE ============

	// UI state
	let selectedVersion = $state('');
	let showFullDescription = $state(false);
	let showInstallPanel = $state(false);
	let selectedProfileId = $state('');

	// Install operation state
	let isInstalling = $state(false);
	let installError = $state('');
	let selectedDependencies = $state<Set<string>>(new Set());
	let modsBeingInstalled = $state<string[]>([]);
	let progressUnlisten: UnlistenFn | null = null;

	// Remove operation state
	let isRemoving = $state(false);

	// ============ DERIVED FROM QUERIES ============

	const mod = $derived(modQuery.data);
	const modInfo = $derived(modInfoQuery.data);
	const versions = $derived(versionsQuery.data ?? []);
	const profiles = $derived((profilesQuery.data ?? []) as Profile[]);
	const hasProfiles = $derived(profiles.length > 0);
	const selectedProfile = $derived(profiles.find((p) => p.id === selectedProfileId));

	// Find this mod in the profile's unified mods (for remove context)
	const unifiedMod = $derived(
		profileId
			? unifiedModsQuery.data?.find((m: UnifiedMod) => m.source === 'managed' && m.mod_id === modId)
			: null
	);

	// ============ VERSION-DEPENDENT QUERIES ============

	// Version info query (depends on selectedVersion)
	const versionInfoQuery = createQuery(() => ({
		...modQueries.versionInfo(modId, selectedVersion),
		enabled: !!selectedVersion
	}));
	const versionInfo = $derived(versionInfoQuery.data);
	const dependencies = $derived(versionInfo?.dependencies ?? []);

	// ============ DEPENDENCY RESOLUTION QUERY ============

	// Create a stable query key from dependencies
	function getDepsQueryKey(deps: ModDependency[]) {
		return deps
			.map((d) => `${d.mod_id}:${d.version_constraint}`)
			.sort()
			.join(',');
	}

	const depsQueryKey = $derived(getDepsQueryKey(dependencies));

	// Resolved dependencies query - used for both display and install panel
	const resolvedDepsQuery = createQuery(() => ({
		queryKey: ['resolved-deps', depsQueryKey] as const,
		queryFn: () => modInstallService.resolveDependencies(dependencies),
		enabled: dependencies.length > 0,
		staleTime: 1000 * 60 * 5
	}));

	const resolvedDeps = $derived(resolvedDepsQuery.data ?? []);
	const installableDependencies = $derived(resolvedDeps.filter((d) => d.type !== 'conflict'));

	// ============ INSTALL PANEL DERIVED STATE ============

	// Check if mod is already installed in selected profile (reactive to profile query)
	const isModInstalledInProfile = $derived(
		selectedProfile?.mods.some((m) => m.mod_id === modId) ?? false
	);

	// Check which dependencies are already installed in selected profile
	const installedDepsInProfile = $derived(
		new Set(selectedProfile?.mods.map((m) => m.mod_id) ?? [])
	);

	// Conflicts in selected profile
	const conflictsInProfile = $derived(
		resolvedDeps
			.filter((d) => d.type === 'conflict')
			.filter((d) => installedDepsInProfile.has(d.mod_id))
	);

	// ============ EFFECTS ============

	// Set default version when versions load
	$effect(() => {
		if (versions.length > 0 && !selectedVersion) {
			const latest = [...versions].sort((a, b) => b.created_at - a.created_at)[0];
			selectedVersion = latest.version;
		}
	});

	// Set default profile when profiles load
	$effect(() => {
		if (profiles.length > 0 && !selectedProfileId) {
			const mostRecent = [...profiles].sort((a, b) => b.created_at - a.created_at)[0];
			selectedProfileId = mostRecent.id;
		}
	});

	// Initialize selected dependencies when resolved deps change
	$effect(() => {
		if (showInstallPanel && resolvedDeps.length > 0) {
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
			showInstallPanel = false;
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

	async function handleRemoveMod() {
		if (!profileId || !unifiedMod) return;

		isRemoving = true;
		try {
			await deleteMod.mutateAsync({ profileId, mod: unifiedMod });
			showSuccess('Mod removed from profile');
			onclose?.();
		} catch (error) {
			showError(error, 'Remove mod');
		} finally {
			isRemoving = false;
		}
	}

	function resetInstallPanel() {
		installError = '';
		selectedDependencies = new Set();
	}

	// ============ HELPERS ============

	const renderedDescription = $derived(
		modInfo?.long_description ? marked.parse(modInfo.long_description, { async: false }) : ''
	);

	const renderedChangelog = $derived(
		versionInfo?.changelog ? marked.parse(versionInfo.changelog, { async: false }) : ''
	);

	const descriptionLength = $derived(modInfo?.long_description?.length ?? 0);
	const shouldTruncate = $derived(descriptionLength > 500);
	const truncatedDescription = $derived(
		shouldTruncate && !showFullDescription
			? marked.parse((modInfo?.long_description ?? '').slice(0, 500) + '...', { async: false })
			: renderedDescription
	);

	function getLinkIcon(type: string) {
		switch (type.toLowerCase()) {
			case 'github':
				return Github;
			case 'discord':
				return MessageCircle;
			default:
				return Globe;
		}
	}

	function formatLinkType(type: string) {
		return type.charAt(0).toUpperCase() + type.slice(1);
	}

	const isLoading = $derived(modQuery.isPending || modInfoQuery.isPending);

	// Cleanup listener on component destroy
	onDestroy(() => {
		if (progressUnlisten) {
			progressUnlisten();
			progressUnlisten = null;
		}
	});
</script>

<div class="flex h-full flex-col">
	<!-- Sticky Header with Controls -->
	<header
		class="sticky top-0 z-10 flex items-center justify-end gap-1.5 border-b border-border/50 bg-muted/90 px-3 py-2 backdrop-blur-sm"
	>
		<Button variant="ghost" size="icon-sm" onclick={() => sidebar.toggleMaximize()}>
			{#if sidebar.isMaximized}
				<Minimize class="size-4" />
			{:else}
				<Maximize class="size-4" />
			{/if}
		</Button>
		<Button variant="ghost" size="icon-sm" onclick={onclose}>
			<X class="size-4" />
		</Button>
	</header>

	<!-- Scrollable Content -->
	<div class="scrollbar-styled min-h-0 flex-1 overflow-y-auto">
		{#if isLoading}
			<!-- Loading Skeleton -->
			<div class="space-y-5 p-5">
				<Skeleton class="mx-auto h-40 w-40 rounded-xl" />
				<div class="flex items-start justify-between gap-4">
					<div class="min-w-0 flex-1 space-y-1.5">
						<Skeleton class="h-7 w-3/4" />
						<Skeleton class="h-4 w-1/3" />
					</div>
				</div>
				<div class="flex gap-4">
					<Skeleton class="h-5 w-28" />
					<Skeleton class="h-5 w-28" />
				</div>
				<div class="flex gap-2">
					<Skeleton class="h-5 w-14 rounded-full" />
					<Skeleton class="h-5 w-18 rounded-full" />
					<Skeleton class="h-5 w-12 rounded-full" />
				</div>
				<div class="space-y-2 pt-2">
					<Skeleton class="h-4 w-full" />
					<Skeleton class="h-4 w-full" />
					<Skeleton class="h-4 w-2/3" />
				</div>
			</div>
		{:else if mod}
			<div class="space-y-5 p-5">
				<!-- Thumbnail -->
				<div class="flex justify-center">
					<div class="relative h-44 w-44 overflow-hidden rounded-xl bg-muted ring-1 ring-border/50">
						{#if mod._links.thumbnail}
							<img src={mod._links.thumbnail} alt={mod.name} class="h-full w-full object-contain" />
						{:else}
							<div class="flex h-full w-full items-center justify-center">
								<ImageOff class="h-12 w-12 text-muted-foreground/30" />
							</div>
						{/if}
					</div>
				</div>

				<!-- Title & Author -->
				<div class="text-center">
					<h2 class="text-xl leading-tight font-bold tracking-tight">{mod.name}</h2>
					<p class="mt-0.5 text-sm text-muted-foreground">by {mod.author}</p>
				</div>

				<!-- Stats Row -->
				<div class="flex flex-wrap items-center justify-center gap-x-4 gap-y-1 text-sm">
					<span class="inline-flex items-center gap-1.5 font-medium">
						<Download size={16} class="text-primary" />
						{mod.downloads.toLocaleString()}
					</span>
					<span class="inline-flex items-center gap-1.5 text-muted-foreground">
						<Clock size={16} />
						{new Date(mod.updated_at).toLocaleDateString()}
					</span>
				</div>

				<!-- Tags -->
				{#if modInfo?.tags && modInfo.tags.length > 0}
					<div class="flex flex-wrap justify-center gap-1.5">
						{#each modInfo.tags as tag (tag)}
							<Badge class="text-xs font-normal">{tag}</Badge>
						{/each}
					</div>
				{/if}

				<!-- Short Description -->
				<p class="text-center text-sm leading-relaxed text-muted-foreground">{mod.description}</p>

				<!-- Divider -->
				<div class="border-t border-border/50"></div>

				<!-- Long Description -->
				{#if modInfo?.long_description}
					<section class="space-y-2">
						<h3 class="text-xs font-semibold tracking-wider text-muted-foreground/70 uppercase">
							About
						</h3>
						<div class="prose-sm">
							<Prose content={truncatedDescription} />
						</div>
						{#if shouldTruncate}
							<button
								type="button"
								class="inline-flex items-center gap-1 text-sm font-medium text-primary hover:text-primary/80"
								onclick={() => (showFullDescription = !showFullDescription)}
							>
								{#if showFullDescription}
									<ChevronUp class="h-4 w-4" />
									Show less
								{:else}
									<ChevronDown class="h-4 w-4" />
									Read more
								{/if}
							</button>
						{/if}
					</section>
				{/if}

				<!-- Version Selector & Changelog -->
				<section class="space-y-3">
					<h3 class="text-xs font-semibold tracking-wider text-muted-foreground/70 uppercase">
						Version
					</h3>

					{#if versionsQuery.isPending}
						<Skeleton class="h-9 w-full" />
					{:else if versions.length > 0}
						<Select.Root bind:value={selectedVersion} type="single">
							<Select.Trigger class="w-full">
								{selectedVersion || 'Select version'}
							</Select.Trigger>
							<Select.Content>
								{#each versions as v (v.version)}
									<Select.Item value={v.version}>
										<span class="flex w-full items-center justify-between gap-3">
											<span>{v.version}</span>
											<span class="text-xs text-muted-foreground">
												{new Date(v.created_at).toLocaleDateString()}
											</span>
										</span>
									</Select.Item>
								{/each}
							</Select.Content>
						</Select.Root>

						<!-- Changelog -->
						{#if versionInfoQuery.isPending}
							<div class="space-y-2 rounded-lg bg-muted/50 p-3">
								<Skeleton class="h-3 w-16" />
								<Skeleton class="h-4 w-full" />
								<Skeleton class="h-4 w-3/4" />
							</div>
						{:else if versionInfo?.changelog}
							<div class="rounded-lg bg-muted/50 p-3">
								<h4
									class="mb-2 text-[11px] font-medium tracking-wider text-muted-foreground/60 uppercase"
								>
									Changelog
								</h4>
								<div class="prose-sm text-sm">
									<Prose content={renderedChangelog} />
								</div>
							</div>
						{/if}
					{:else}
						<p class="text-sm text-muted-foreground">No versions available</p>
					{/if}
				</section>

				<!-- Dependencies (read-only display) -->
				{#if dependencies.length > 0}
					<section class="space-y-2">
						<h3 class="text-xs font-semibold tracking-wider text-muted-foreground/70 uppercase">
							Dependencies
						</h3>
						{#if resolvedDepsQuery.isPending}
							<div class="space-y-1.5">
								<Skeleton class="h-10 w-full rounded-lg" />
								<Skeleton class="h-10 w-full rounded-lg" />
							</div>
						{:else}
							<div class="space-y-1.5">
								{#each resolvedDeps as dep (dep.mod_id)}
									<div
										class="flex items-center justify-between rounded-lg bg-muted/50 px-3 py-2 text-sm"
									>
										<span class="font-medium">{dep.modName}</span>
										<span class="flex items-center gap-2">
											<span class="text-xs text-muted-foreground">v{dep.resolvedVersion}</span>
											<Badge
												variant={dep.type === 'required'
													? 'default'
													: dep.type === 'conflict'
														? 'destructive'
														: 'secondary'}
												class="text-[10px]"
											>
												{dep.type}
											</Badge>
										</span>
									</div>
								{/each}
							</div>
						{/if}
					</section>
				{/if}

				<!-- External Links -->
				{#if modInfo?.links && modInfo.links.length > 0}
					<section class="space-y-2">
						<h3 class="text-xs font-semibold tracking-wider text-muted-foreground/70 uppercase">
							Links
						</h3>
						<div class="flex flex-wrap gap-2">
							{#each modInfo.links as link, i (`${link.type}-${link.url}-${i}`)}
								{@const Icon = getLinkIcon(link.type)}
								<Button variant="outline" size="sm" href={link.url} class="h-8 gap-1.5 text-xs">
									<Icon class="h-4 w-4" />
									{formatLinkType(link.type)}
									<ExternalLink class="h-3 w-3 opacity-40" />
								</Button>
							{/each}
						</div>
					</section>
				{/if}

				<!-- License Footer -->
				{#if modInfo?.license}
					<p class="pt-2 text-[11px] text-muted-foreground/60">
						Licensed under {modInfo.license}
					</p>
				{/if}

				<!-- Spacer for sticky footer -->
				<div class="h-20"></div>
			</div>
		{:else}
			<!-- Error State -->
			<div class="flex h-full flex-col items-center justify-center p-6 text-center">
				<ImageOff class="mb-4 h-12 w-12 text-muted-foreground/30" />
				<h3 class="mb-1 font-semibold">Mod not found</h3>
				<p class="text-sm text-muted-foreground">The requested mod could not be loaded.</p>
			</div>
		{/if}
	</div>

	<!-- Sticky Footer Action Bar -->
	{#if mod && !isLoading}
		<div
			class="sticky bottom-0 border-t border-border/50 bg-background/95 backdrop-blur-sm transition-all duration-300"
		>
			{#if profileId && unifiedMod}
				<!-- Profile context: Remove button -->
				<div class="p-4">
					<Button
						variant="destructive"
						class="w-full gap-2"
						onclick={handleRemoveMod}
						disabled={isRemoving}
					>
						{#if isRemoving}
							<Loader2 class="h-4 w-4 animate-spin" />
							Removing...
						{:else}
							<Trash2 class="h-4 w-4" />
							Remove from Profile
						{/if}
					</Button>
				</div>
			{:else if hasProfiles}
				<!-- Install panel -->
				{#if showInstallPanel}
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
											{@const isInstalled = selectedProfile?.mods.some(
												(m) => m.mod_id === dep.mod_id
											)}
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
														<span class="text-[11px] text-muted-foreground"
															>v{dep.resolvedVersion}</span
														>
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
													<Loader2 class="h-3.5 w-3.5 animate-spin text-primary" />
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
													<Loader2 class="h-3.5 w-3.5 animate-spin text-muted-foreground" />
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
				{/if}

				<!-- Action buttons -->
				<div class="flex gap-2 p-4">
					{#if showInstallPanel}
						<Button
							variant="outline"
							class="flex-1"
							onclick={() => {
								showInstallPanel = false;
								resetInstallPanel();
							}}
							disabled={isInstalling}
						>
							Cancel
						</Button>
						<Button
							class="flex-1 gap-2"
							onclick={handleInstall}
							disabled={isInstalling ||
								!selectedProfileId ||
								!selectedVersion ||
								isModInstalledInProfile}
						>
							{#if isInstalling}
								<Loader2 class="h-4 w-4 animate-spin" />
								Installing...
							{:else}
								<Download size={16} />
								Install
							{/if}
						</Button>
					{:else}
						<Button class="w-full gap-2" onclick={() => (showInstallPanel = true)}>
							<Download size={16} />
							Install to Profile
						</Button>
					{/if}
				</div>
			{:else}
				<!-- No profiles -->
				<div class="p-4">
					<Button disabled class="w-full opacity-50">
						<Package class="mr-2 h-4 w-4" />
						No profiles available
					</Button>
				</div>
			{/if}
		</div>
	{/if}
</div>
