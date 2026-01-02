<script lang="ts">
	import * as Dialog from '$lib/components/ui/dialog';
	import { Button } from '$lib/components/ui/button';
	import * as Select from '$lib/components/ui/select';
	import { Label } from '$lib/components/ui/label';
	import { Switch } from '$lib/components/ui/switch';
	import { Plus } from '@lucide/svelte';
	import { createQuery, useQueryClient } from '@tanstack/svelte-query';
	import { profileQueries } from '../queries';
	import { modQueries } from '$lib/features/mods/queries';
	import { modInstallService, type DependencyWithMeta } from '../mod-install-service';
	import { profileService } from '../profile-service';
	import { type ModDependency } from '$lib/features/mods/schema';
	import { AlertTriangle } from '@lucide/svelte';

	const queryClient = useQueryClient();

	let { modId }: { modId: string } = $props();

	let open = $state(false);
	let selectedProfileId = $state('');
	let selectedVersion = $state('');
	let isInstalling = $state(false);
	let error = $state('');
	let resolvedDependencies = $state<DependencyWithMeta[]>([]);
	let selectedDependencies = $state<Set<string>>(new Set());
	let isLoadingDeps = $state(false);

	const profilesQuery = createQuery(() => ({
		...profileQueries.all(),
		enabled: open
	}));
	const versionsQuery = createQuery(() => ({
		...modQueries.versions(modId),
		enabled: open
	}));
	const versionInfoQuery = createQuery(() => ({
		...modQueries.versionInfo(modId, selectedVersion),
		enabled: open && !!selectedVersion
	}));

	const profiles = $derived(profilesQuery.data ?? []);
	const versions = $derived(versionsQuery.data ?? []);
	const selectedProfile = $derived(profiles.find((p) => p.id === selectedProfileId));
	const installableDependencies = $derived(
		resolvedDependencies.filter((d) => d.type !== 'conflict')
	);
	const conflictsInProfile = $derived(
		resolvedDependencies
			.filter((d) => d.type === 'conflict')
			.filter((d) => selectedProfile?.mods.some((m) => m.mod_id === d.mod_id))
	);

	$effect(() => {
		if (profiles.length > 0 && !selectedProfile) {
			const lastLaunched = [...profiles].sort((a, b) => b.created_at - a.created_at)[0];
			selectedProfileId = lastLaunched.id;
		}
		if (versions.length > 0 && !selectedVersion) {
			const latest = [...versions].sort((a, b) => b.created_at - a.created_at)[0];
			selectedVersion = latest.version;
		}
	});

	$effect(() => {
		const versionInfo = versionInfoQuery.data;
		if (versionInfo && versionInfo.dependencies.length > 0) {
			loadDependencies(versionInfo.dependencies);
		} else if (versionInfo) {
			resolvedDependencies = [];
			selectedDependencies = new Set();
		}
	});

	async function loadDependencies(dependencies: ModDependency[]) {
		isLoadingDeps = true;
		try {
			const resolved = await modInstallService.resolveDependencies(dependencies);
			resolvedDependencies = resolved;
			selectedDependencies = new Set(
				resolved.filter((d) => d.type !== 'conflict').map((d) => d.mod_id)
			);
		} catch {
			resolvedDependencies = [];
			selectedDependencies = new Set();
		} finally {
			isLoadingDeps = false;
		}
	}

	function toggleDependency(modId: string) {
		selectedDependencies = new Set(
			selectedDependencies.has(modId)
				? [...selectedDependencies].filter((id) => id !== modId)
				: [...selectedDependencies, modId]
		);
	}

	async function handleInstall() {
		if (!selectedProfile || !selectedVersion) return;
		try {
			isInstalling = true;
			error = '';

			const modsToInstall = [{ modId, version: selectedVersion }];

			for (const dep of installableDependencies) {
				if (
					selectedDependencies.has(dep.mod_id) &&
					!selectedProfile.mods.some((m) => m.mod_id === dep.mod_id)
				) {
					modsToInstall.push({ modId: dep.mod_id, version: dep.resolvedVersion });
				}
			}

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

			await queryClient.invalidateQueries({ queryKey: ['profiles'] });
			open = false;
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to install';
		} finally {
			isInstalling = false;
		}
	}

	function reset() {
		selectedProfileId = '';
		selectedVersion = '';
		error = '';
		resolvedDependencies = [];
		selectedDependencies = new Set();
	}
</script>

<Dialog.Root bind:open onOpenChange={(v) => !v && reset()}>
	<Dialog.Trigger>
		<Button size="sm">Install <Plus class="ml-2 h-4 w-4" /></Button>
	</Dialog.Trigger>
	<Dialog.Content>
		<Dialog.Header>
			<Dialog.Title>Add Mod to Profile</Dialog.Title>
		</Dialog.Header>

		<div class="space-y-4 py-4">
			<div class="space-y-2">
				<Label>Profile</Label>
				<Select.Root bind:value={selectedProfileId} type="single" disabled={isInstalling}>
					<Select.Trigger>{selectedProfile?.name ?? 'Select profile'}</Select.Trigger>
					<Select.Content>
						{#each profiles as p (p.id)}
							<Select.Item value={p.id}>{p.name}</Select.Item>
						{/each}
					</Select.Content>
				</Select.Root>
			</div>

			<div class="space-y-2">
				<Label>Version</Label>
				<Select.Root bind:value={selectedVersion} type="single" disabled={isInstalling}>
					<Select.Trigger>{selectedVersion || 'Select version'}</Select.Trigger>
					<Select.Content>
						{#each versions as v (v.version)}
							<Select.Item value={v.version}>{v.version}</Select.Item>
						{/each}
					</Select.Content>
				</Select.Root>
			</div>

			{#if isLoadingDeps}
				<div class="space-y-2">
					<Label>Dependencies</Label>
					<p class="text-sm text-muted-foreground">Loading dependencies...</p>
				</div>
			{:else if installableDependencies.length > 0}
				<div class="space-y-2">
					<Label>Dependencies</Label>
					<div class="space-y-2 rounded-md border p-3">
						{#each installableDependencies as dep (dep.mod_id)}
							{@const isInstalled = selectedProfile?.mods.some((m) => m.mod_id === dep.mod_id)}
							<div class="flex items-center justify-between">
								<div class="flex items-center gap-2">
									<Switch
										checked={selectedDependencies.has(dep.mod_id)}
										onCheckedChange={() => toggleDependency(dep.mod_id)}
										disabled={isInstalling || isInstalled}
									/>
									<span class="text-sm">{dep.modName}</span>
									<span class="text-xs text-muted-foreground">v{dep.resolvedVersion}</span>
									{#if isInstalled}
										<span class="rounded-full bg-muted px-2 py-0.5 text-xs text-muted-foreground"
											>Installed</span
										>
									{/if}
								</div>
								<span
									class="text-xs {dep.type === 'required'
										? 'text-destructive'
										: 'text-muted-foreground'}"
								>
									{dep.type.charAt(0).toUpperCase() + dep.type.slice(1)}
								</span>
							</div>
						{/each}
					</div>
				</div>
			{/if}

			{#if conflictsInProfile.length > 0}
				<div class="rounded-md border border-destructive/50 bg-destructive/10 p-3">
					<div class="flex items-start gap-2">
						<AlertTriangle class="mt-0.5 h-4 w-4 shrink-0 text-destructive" />
						<div class="space-y-1">
							<p class="text-sm font-medium text-destructive">Conflicts Detected</p>
							<p class="text-xs text-destructive/80">
								This mod conflicts with {conflictsInProfile.length === 1 ? 'a mod' : 'mods'} already in
								this profile:
							</p>
							<ul class="list-disc pl-4 text-xs text-destructive/80">
								{#each conflictsInProfile as conflict (conflict.mod_id)}
									<li>{conflict.modName}</li>
								{/each}
							</ul>
						</div>
					</div>
				</div>
			{/if}

			{#if error}<p class="text-sm text-destructive">{error}</p>{/if}

			<div class="flex justify-end gap-2">
				<Button variant="outline" onclick={() => (open = false)} disabled={isInstalling}
					>Cancel</Button
				>
				<Button
					onclick={handleInstall}
					disabled={isInstalling || !selectedProfileId || !selectedVersion}
				>
					{isInstalling ? 'Installing...' : 'Install'}
				</Button>
			</div>
		</div>
	</Dialog.Content>
</Dialog.Root>
