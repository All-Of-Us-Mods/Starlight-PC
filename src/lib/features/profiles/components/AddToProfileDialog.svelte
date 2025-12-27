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
	import type { Profile } from '../schema';

	let { modId }: { modId: string } = $props();

	let open = $state(false);
	let selectedProfileId = $state<string>('');
	let selectedVersion = $state<string>('');
	let isInstalling = $state(false);
	let error = $state('');

	const profilesQuery = createQuery(() => profileQueries.all());
	const versionsQuery = createQuery(() => ({
		...modQueries.versions(modId),
		enabled: open
	}));

	const profiles = $derived((profilesQuery.data ?? []) as Profile[]);
	const versions = $derived(versionsQuery.data ?? []);

	const selectedProfile = $derived(
		selectedProfileId ? profiles.find((p) => p.id === selectedProfileId) : undefined
	);

	async function handleInstall() {
		if (!selectedProfileId || !selectedVersion) return;

		const profile = selectedProfile;
		if (!profile) {
			error = 'Profile not found';
			return;
		}

		error = '';
		try {
			isInstalling = true;
			await modInstallService.installModToProfile(modId, selectedVersion, profile.path);

			await profileService.addModToProfile(selectedProfileId, modId, selectedVersion);

			selectedProfileId = '';
			selectedVersion = '';
			open = false;
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to install mod';
		} finally {
			isInstalling = false;
		}
	}

	function onOpenChange(isOpen: boolean) {
		open = isOpen;
		if (!isOpen) {
			selectedProfileId = '';
			selectedVersion = '';
			error = '';
		}
	}
</script>

<Dialog.Root bind:open {onOpenChange}>
	<Dialog.Trigger>
		<Button size="sm">
			<Plus class="mr-2 h-4 w-4" />
			Add to Profile
		</Button>
	</Dialog.Trigger>
	<Dialog.Content>
		<Dialog.Header>
			<Dialog.Title>Add Mod to Profile</Dialog.Title>
			<Dialog.Description>Select a profile and version to install this mod.</Dialog.Description>
		</Dialog.Header>

		<div class="space-y-4 py-4">
			<div class="space-y-2">
				<Label for="profile">Profile</Label>
				<Select.Root bind:value={selectedProfileId} type="single" disabled={isInstalling}>
					<Select.Trigger id="profile">
						{selectedProfile?.name ?? 'Select a profile'}
					</Select.Trigger>
					<Select.Content>
						{#each profiles as profile (profile.id)}
							<Select.Item value={profile.id}>{profile.name}</Select.Item>
						{/each}
					</Select.Content>
				</Select.Root>
			</div>

			<div class="space-y-2">
				<Label for="version">Version</Label>
				<Select.Root
					bind:value={selectedVersion}
					type="single"
					disabled={isInstalling || !selectedProfileId}
				>
					<Select.Trigger id="version">
						{selectedVersion ?? 'Select a version'}
					</Select.Trigger>
					<Select.Content>
						{#each versions as version (version.id)}
							<Select.Item value={version.version}>{version.version}</Select.Item>
						{/each}
					</Select.Content>
				</Select.Root>
			</div>

			{#if error}
				<p class="text-sm font-medium text-destructive">{error}</p>
			{/if}

			<div class="flex justify-end gap-2">
				<Dialog.Close>
					<Button variant="outline" disabled={isInstalling}>Cancel</Button>
				</Dialog.Close>
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
