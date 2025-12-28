<script lang="ts">
	import * as Dialog from '$lib/components/ui/dialog';
	import { Button } from '$lib/components/ui/button';
	import * as Select from '$lib/components/ui/select';
	import { Label } from '$lib/components/ui/label';
	import { Plus } from '@lucide/svelte';
	import { createQuery } from '@tanstack/svelte-query';
	import { profileQueries, modQueries } from '../queries';
	import { modInstallService } from '../mod-install-service';
	import { profileService } from '../profile-service';

	let { modId }: { modId: string } = $props();

	let open = $state(false);
	let selectedProfileId = $state('');
	let selectedVersion = $state('');
	let isInstalling = $state(false);
	let error = $state('');

	const profilesQuery = createQuery(() => ({
		...profileQueries.all(),
		enabled: open
	}));
	const versionsQuery = createQuery(() => ({
		...modQueries.versions(modId),
		enabled: open
	}));

	const profiles = $derived(profilesQuery.data ?? []);
	const versions = $derived(versionsQuery.data ?? []);
	const selectedProfile = $derived(profiles.find((p) => p.id === selectedProfileId));

	// Automatically select the latest version when data arrives
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

	async function handleInstall() {
		if (!selectedProfile || !selectedVersion) return;
		try {
			isInstalling = true;
			error = '';
			await modInstallService.installModToProfile(modId, selectedVersion, selectedProfile.path);
			await profileService.addModToProfile(selectedProfileId, modId, selectedVersion);
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
