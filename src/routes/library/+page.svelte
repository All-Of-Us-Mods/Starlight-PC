<script lang="ts">
	import { Library, Plus } from '@lucide/svelte';
	import { Upload } from '@jis3r/icons';
	import PageHeader from '$lib/components/shared/PageHeader.svelte';
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
	import { profileActions } from '$lib/features/profiles/actions';
	import { rememberInstallTarget } from '$lib/features/mods/state/install-target.svelte';
	import type { Profile } from '$lib/features/profiles/schema';
	import { showError, showSuccess } from '$lib/utils/toast';
	import { profileUnifiedModsKey, profilesQueryKey } from '$lib/features/profiles/profile-keys';
	import CreateProfileDialogContainer from '$lib/features/profiles/containers/CreateProfileDialogContainer.svelte';
	import LibraryQuickActions from '$lib/features/profiles/components/library/LibraryQuickActions.svelte';
	import LibraryProfilesSection from '$lib/features/profiles/components/library/LibraryProfilesSection.svelte';

	const queryClient = useQueryClient();
	const profilesQuery = createQuery(() => profileQueries.all());
	const settingsQuery = createQuery(() => settingsQueries.get());
	const launchProfileMutation = createMutation(() => profileActions.launchProfile(queryClient));
	const launchVanillaMutation = createMutation(() => profileActions.launchVanilla(queryClient));
	const stopProfileInstancesMutation = createMutation<number, Error, string>(() =>
		profileActions.stopProfileInstances()
	);
	const deleteProfile = createMutation(() => profileActions.delete(queryClient));
	const importProfileZip = createMutation(() => profileActions.importZip(queryClient));
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
			await launchVanillaMutation.mutateAsync();
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
				p.id === profile.id ? Object.assign({}, p, { last_launched_at: Date.now() }) : p
			)
		);

		try {
			await launchProfileMutation.mutateAsync(profile);
			rememberInstallTarget(profile.id, 'launch');
		} catch (e) {
			queryClient.setQueryData(profilesQueryKey, previousProfiles);
			showError(e);
		}
	}

	async function handleStopProfile(profile: Profile) {
		try {
			const stoppedCount = await stopProfileInstancesMutation.mutateAsync(profile.id);
			showSuccess(
				stoppedCount === 1
					? `Stopped "${profile.name}"`
					: `Stopped ${stoppedCount} instances for "${profile.name}"`
			);
		} catch (error) {
			showError(error, 'Stop profile');
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
			if (imported.length === 1) {
				showSuccess(`Profile "${imported[0].name}" imported`);
			} else {
				showSuccess(`${imported.length} profiles imported`);
			}
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
	<CreateProfileDialogContainer bind:open={createDialogOpen} />

	<LibraryQuickActions {isLaunchingVanilla} onLaunchVanilla={handleLaunchVanilla} />
	<LibraryProfilesSection
		isPending={profilesQuery.isPending}
		{profiles}
		{allowMultiInstanceLaunch}
		onCreateProfile={() => (createDialogOpen = true)}
		onLaunchProfile={handleLaunchProfile}
		onStopProfile={handleStopProfile}
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
