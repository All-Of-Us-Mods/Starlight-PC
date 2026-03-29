<script lang="ts" module>
	import type { Profile } from '$lib/features/profiles/schema';

	export interface ProfileDialogsProps {
		profile: Profile;
		deleteDialogOpen: boolean;
		renameDialogOpen: boolean;
	}
</script>

<script lang="ts">
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { createMutation, useQueryClient } from '@tanstack/svelte-query';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { profileActions } from '$lib/features/profiles/actions';
	import { profileUnifiedModsKey } from '$lib/features/profiles/profile-keys';
	import { showError, showSuccess } from '$lib/utils/toast';
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
	import * as Dialog from '$lib/components/ui/dialog';

	let {
		profile,
		deleteDialogOpen = $bindable(),
		renameDialogOpen = $bindable()
	}: ProfileDialogsProps = $props();

	const queryClient = useQueryClient();
	const deleteProfile = createMutation(() => profileActions.delete(queryClient));
	const renameProfile = createMutation(() => profileActions.rename(queryClient));

	let newProfileName = $state('');
	let renameError = $state('');

	function handleRenameOpenChange(isOpen: boolean) {
		renameDialogOpen = isOpen;
		if (isOpen) {
			newProfileName = profile.name;
			renameError = '';
		}
	}

	async function handleDeleteProfile() {
		deleteDialogOpen = false;
		try {
			await deleteProfile.mutateAsync(profile.id);
			queryClient.removeQueries({ queryKey: profileUnifiedModsKey(profile.id) });
			showSuccess(`Profile "${profile.name}" deleted`);
			goto(resolve('/library'));
		} catch (error) {
			showError(error);
		}
	}

	async function handleRenameProfile() {
		if (!newProfileName.trim()) return;
		renameError = '';
		try {
			await renameProfile.mutateAsync({ profileId: profile.id, newName: newProfileName });
			showSuccess('Profile renamed');
			renameDialogOpen = false;
		} catch (error) {
			renameError = error instanceof Error ? error.message : 'Failed to rename';
		}
	}
</script>

<AlertDialog bind:open={deleteDialogOpen}>
	<AlertDialogContent>
		<AlertDialogHeader>
			<AlertDialogTitle>Delete Profile?</AlertDialogTitle>
			<AlertDialogDescription>
				Are you sure you want to delete <strong>{profile.name}</strong>? This action cannot be undone
				and will delete all files associated with this profile.
			</AlertDialogDescription>
		</AlertDialogHeader>
		<AlertDialogFooter>
			<AlertDialogCancel onclick={() => (deleteDialogOpen = false)}>Cancel</AlertDialogCancel>
			<AlertDialogAction
				onclick={handleDeleteProfile}
				class="bg-destructive text-destructive-foreground hover:bg-destructive/90"
				disabled={deleteProfile.isPending}
			>
				Delete Profile
			</AlertDialogAction>
		</AlertDialogFooter>
	</AlertDialogContent>
</AlertDialog>

<Dialog.Root bind:open={renameDialogOpen} onOpenChange={handleRenameOpenChange}>
	<Dialog.Content>
		<Dialog.Header>
			<Dialog.Title>Rename Profile</Dialog.Title>
			<Dialog.Description>Enter a new name for this profile.</Dialog.Description>
		</Dialog.Header>

		<div class="space-y-4 py-4">
			<Input
				bind:value={newProfileName}
				placeholder="Profile name"
				disabled={renameProfile.isPending}
			/>
			{#if renameError}
				<p class="text-sm text-destructive">{renameError}</p>
			{/if}
		</div>

		<Dialog.Footer>
			<Button variant="outline" onclick={() => (renameDialogOpen = false)}>Cancel</Button>
			<Button onclick={handleRenameProfile} disabled={renameProfile.isPending || !newProfileName.trim()}>
				{#if renameProfile.isPending}
					<div
						class="mr-2 h-4 w-4 animate-spin rounded-full border-2 border-current border-t-transparent"
					></div>
					Renaming...
				{:else}
					Rename
				{/if}
			</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>
