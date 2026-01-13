<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { Badge } from '$lib/components/ui/badge';
	import { Skeleton } from '$lib/components/ui/skeleton';
	import * as Select from '$lib/components/ui/select';
	import Prose from '$lib/components/shared/Prose.svelte';
	import SidebarHeader from '$lib/components/shared/SidebarHeader.svelte';
	import InstallPanel from './InstallPanel.svelte';
	import { marked } from 'marked';
	import { ImageOff, Download, Clock } from '@jis3r/icons';
	import {
		ExternalLink,
		Github,
		Globe,
		MessageCircle,
		ChevronDown,
		ChevronUp,
		Trash2,
		LoaderCircle
	} from '@lucide/svelte';
	import { createMutation, createQuery, useQueryClient } from '@tanstack/svelte-query';
	import { modQueries } from '../queries';
	import { profileQueries } from '$lib/features/profiles/queries';
	import { profileMutations } from '$lib/features/profiles/mutations';
	import { showError, showSuccess } from '$lib/utils/toast';
	import type { UnifiedMod, Profile } from '$lib/features/profiles/schema';

	interface Props {
		modId: string;
		profileId?: string;
		onclose?: () => void;
	}

	let { modId, profileId, onclose }: Props = $props();

	const queryClient = useQueryClient();
	const deleteMod = createMutation(() => profileMutations.deleteUnifiedMod(queryClient));

	// ============ QUERIES ============

	const modQuery = createQuery(() => modQueries.byId(modId));
	const modInfoQuery = createQuery(() => modQueries.info(modId));
	const versionsQuery = createQuery(() => modQueries.versions(modId));
	const profilesQuery = createQuery(() => profileQueries.all());
	const unifiedModsQuery = createQuery(() => ({
		...profileQueries.unifiedMods(profileId ?? ''),
		enabled: !!profileId
	}));

	// ============ STATE ============

	let selectedVersion = $state('');
	let showFullDescription = $state(false);
	let showInstallPanel = $state(false);
	let isRemoving = $state(false);
	let hasSetProfileVersion = $state(false);

	// ============ DERIVED ============

	const mod = $derived(modQuery.data);
	const modInfo = $derived(modInfoQuery.data);
	const versions = $derived(versionsQuery.data ?? []);
	const profiles = $derived((profilesQuery.data ?? []) as Profile[]);
	const hasProfiles = $derived(profiles.length > 0);

	// Find this mod in the profile's unified mods (for remove context)
	const unifiedMod = $derived(
		profileId
			? unifiedModsQuery.data?.find((m: UnifiedMod) => m.source === 'managed' && m.mod_id === modId)
			: null
	);

	// Version info query (depends on selectedVersion)
	const versionInfoQuery = createQuery(() => ({
		...modQueries.versionInfo(modId, selectedVersion),
		enabled: !!selectedVersion
	}));
	const versionInfo = $derived(versionInfoQuery.data);
	const dependencies = $derived(versionInfo?.dependencies ?? []);

	// Resolved dependencies for display with names
	const resolvedDepsQuery = createQuery(() => modQueries.resolvedDependencies(dependencies));
	const resolvedDeps = $derived(resolvedDepsQuery.data ?? []);

	const isLoading = $derived(modQuery.isPending || modInfoQuery.isPending);

	// Description helpers
	function safeParseMarkdown(content: string | undefined): string {
		if (!content) return '';
		try {
			return marked.parse(content, { async: false });
		} catch {
			return content;
		}
	}

	const renderedDescription = $derived(safeParseMarkdown(modInfo?.long_description));
	const renderedChangelog = $derived(safeParseMarkdown(versionInfo?.changelog));
	const descriptionLength = $derived(modInfo?.long_description?.length ?? 0);
	const shouldTruncate = $derived(descriptionLength > 500);
	const truncatedDescription = $derived(
		shouldTruncate && !showFullDescription
			? safeParseMarkdown((modInfo?.long_description ?? '').slice(0, 500) + '...')
			: renderedDescription
	);

	// ============ EFFECTS ============

	// Set default version when versions load
	$effect(() => {
		if (versions.length > 0 && !selectedVersion) {
			const latest = [...versions].sort((a, b) => b.created_at - a.created_at)[0];
			selectedVersion = latest.version;
		}
	});

	// Update selected version to match installed version when profile context is provided initially
	$effect(() => {
		if (
			unifiedMod &&
			unifiedMod.source === 'managed' &&
			unifiedMod.version &&
			!hasSetProfileVersion
		) {
			selectedVersion = unifiedMod.version;
			hasSetProfileVersion = true;
		}
	});

	// ============ HANDLERS ============

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

	// ============ HELPERS ============

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
</script>

<div class="flex h-full flex-col">
	<SidebarHeader {onclose} />

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
		{:else if modQuery.isError || modInfoQuery.isError}
			<div class="flex h-full flex-col items-center justify-center p-6 text-center">
				<ImageOff class="mb-4 h-12 w-12 text-muted-foreground/30" />
				<h3 class="mb-1 font-semibold">Failed to load mod</h3>
				<p class="text-sm text-muted-foreground">There was an error loading the mod details.</p>
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
			class="shrink-0 border-t border-border/50 bg-background/95 backdrop-blur-sm transition-all duration-300"
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
							<LoaderCircle class="h-4 w-4 animate-spin" />
							Removing...
						{:else}
							<Trash2 class="h-4 w-4" />
							Remove from Profile
						{/if}
					</Button>
				</div>
			{:else if hasProfiles}
				{#if showInstallPanel}
					<InstallPanel
						{modId}
						{selectedVersion}
						{dependencies}
						onInstallComplete={() => (showInstallPanel = false)}
						onCancel={() => (showInstallPanel = false)}
					/>
				{:else}
					<div class="p-4">
						<Button class="w-full gap-2" onclick={() => (showInstallPanel = true)}>
							<Download size={16} />
							Install to Profile
						</Button>
					</div>
				{/if}
			{:else}
				<InstallPanel
					{modId}
					{selectedVersion}
					{dependencies}
					onInstallComplete={() => {}}
					onCancel={() => {}}
				/>
			{/if}
		</div>
	{/if}
</div>
