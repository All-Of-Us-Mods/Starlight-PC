<script lang="ts">
	import { Library, Plus } from '@lucide/svelte';
	import { Upload } from '@jis3r/icons';
	import PageHeader from '$lib/components/shared/PageHeader.svelte';
	import CreateProfileDialog from '$lib/features/profiles/components/CreateProfileDialog.svelte';
	import { Button } from '$lib/components/ui/button';
	import { open as openDialog } from '@tauri-apps/plugin-dialog';
	import {
		AlertDialog,
		AlertDialogAction,
		AlertDialogCancel,
		AlertDialogContent,
		AlertDialogDescription,
		AlertDialogFooter,
		AlertDialogHeader,
		AlertDialogTitle
	} from '$lib/components/ui/alert-dialog';
	import { createMutation, createQuery, useQueryClient } from '@tanstack/svelte-query';
	import { settingsQueries } from '$lib/features/settings/queries';
	import { profileQueries } from '$lib/features/profiles/queries';
	import { launchService } from '$lib/features/profiles/launch-service';
	import { profileMutations } from '$lib/features/profiles/mutations';
	import type { Profile } from '$lib/features/profiles/schema';
	import { showError, showSuccess } from '$lib/utils/toast';
	import { profileUnifiedModsKey, profilesQueryKey } from '$lib/features/profiles/profile-keys';
	import LibraryQuickActions from './_components/LibraryQuickActions.svelte';
	import LibraryProfilesSection from './_components/LibraryProfilesSection.svelte';

	const queryClient = useQueryClient();
	const profilesQuery = createQuery(() => profileQueries.all());
	const settingsQuery = createQuery(() => settingsQueries.get());
	const updateLastLaunched = createMutation(() => profileMutations.updateLastLaunched(queryClient));
	const deleteProfile = createMutation(() => profileMutations.delete(queryClient));
	const importProfileZip = createMutation(() => profileMutations.importZip(queryClient));
	const profiles = $derived((profilesQuery.data ?? []) as Profile[]);
	const allowMultiInstanceLaunch = $derived(
		(settingsQuery.data?.allow_multi_instance_launch ?? false) as boolean
	);

	let deleteDialogOpen = $state(false);
	let createDialogOpen = $state(false);
	let profileToDelete = $state<Profile | null>(null);
	let isLaunchingVanilla = $state(false);
	let isImporting = $state(false);

	async function handleLaunchVanilla() {
		isLaunchingVanilla = true;
		try {
			await launchService.launchVanilla();
		} catch (e) {
			showError(e);
		} finally {
			isLaunchingVanilla = false;
		}
	}

	async function handleLaunchProfile(profile: Profile) {
		const previousProfiles = queryClient.getQueryData<Profile[]>(profilesQueryKey);

		queryClient.setQueryData(profilesQueryKey, (old = []) =>
			(old as Profile[]).map((p) =>
				p.id === profile.id ? { ...p, last_launched_at: Date.now() } : p
			)
		);

		try {
			await launchService.launchProfile(profile);
			await updateLastLaunched.mutateAsync(profile.id);
		} catch (e) {
			queryClient.setQueryData(profilesQueryKey, previousProfiles);
			showError(e);
		}
	}

	async function handleImportProfile() {
		try {
			isImporting = true;
			const selected = await openDialog({
				multiple: false,
				directory: false,
				title: 'Import Profile ZIP',
				filters: [{ name: 'ZIP Archive', extensions: ['zip'] }]
			});
			if (!selected) return;

			const imported = await importProfileZip.mutateAsync(selected);
			showSuccess(`Profile "${imported.name}" imported`);
		} catch (e) {
			showError(e, 'Import profile');
		} finally {
			isImporting = false;
		}
	}

	function confirmDeleteProfile(profileId: string) {
		const profile = profiles.find((p) => p.id === profileId);
		if (profile) {
			profileToDelete = profile;
			deleteDialogOpen = true;
		}
	}

	async function handleDeleteProfile() {
		if (!profileToDelete) return;

		const profileId = profileToDelete.id;
		const profileName = profileToDelete.name;
		deleteDialogOpen = false;

		const previousProfiles = queryClient.getQueryData<Profile[]>(profilesQueryKey);

		// Optimistic update
		queryClient.setQueryData(profilesQueryKey, (old = []) =>
			(old as Profile[]).filter((p) => p.id !== profileId)
		);

		try {
			await deleteProfile.mutateAsync(profileId);
			// Also remove any cached unified-mods for this profile
			queryClient.removeQueries({ queryKey: profileUnifiedModsKey(profileId) });
			showSuccess(`Profile "${profileName}" deleted`);
		} catch (e) {
			queryClient.setQueryData(profilesQueryKey, previousProfiles);
			showError(e);
		} finally {
			profileToDelete = null;
		}
	}

	function cancelDelete() {
		deleteDialogOpen = false;
		profileToDelete = null;
	}
</script>

<div class="px-10 py-8">
	<PageHeader
		title="Library"
		description="Manage your profiles and launch the game."
		icon={Library}
	>
		<div>
			<Button variant="outline" onclick={handleImportProfile} disabled={isImporting}>
				<Upload class="mr-2 h-4 w-4" />
				{isImporting ? 'Importing...' : 'Import Profile'}
			</Button>
			<Button onclick={() => (createDialogOpen = true)}>
				<Plus class="mr-2 h-4 w-4" />
				Create Profile
			</Button>
		</div>
	</PageHeader>
	<CreateProfileDialog bind:open={createDialogOpen} />

	<LibraryQuickActions {isLaunchingVanilla} onLaunchVanilla={handleLaunchVanilla} />
	<LibraryProfilesSection
		isPending={profilesQuery.isPending}
		{profiles}
		{allowMultiInstanceLaunch}
		onCreateProfile={() => (createDialogOpen = true)}
		onLaunchProfile={handleLaunchProfile}
		onDeleteProfile={confirmDeleteProfile}
	/>
</div>

<AlertDialog bind:open={deleteDialogOpen}>
	<AlertDialogContent>
		<AlertDialogHeader>
			<AlertDialogTitle>Delete Profile?</AlertDialogTitle>
			<AlertDialogDescription>
				Are you sure you want to delete <strong>{profileToDelete?.name}</strong>? This action cannot
				be undone and will delete all files associated with this profile.
			</AlertDialogDescription>
		</AlertDialogHeader>
		<AlertDialogFooter>
			<AlertDialogCancel onclick={cancelDelete}>Cancel</AlertDialogCancel>
			<AlertDialogAction
				onclick={handleDeleteProfile}
				class="bg-destructive text-destructive-foreground hover:bg-destructive/90"
			>
				Delete Profile
			</AlertDialogAction>
		</AlertDialogFooter>
	</AlertDialogContent>
</AlertDialog>
