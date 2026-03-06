<script lang="ts" module>
	import type { Profile } from '$lib/features/profiles/schema';

	export interface ProfileDialogsProps {
		profile: Profile | null;
		deleteDialogOpen: boolean;
		renameDialogOpen: boolean;
		deleteModDialogOpen: boolean;
		newProfileName: string;
		renameError: string;
		renamePending: boolean;
		onNewProfileNameInput: (event: Event) => void;
		onCancelDeleteProfile: () => void;
		onConfirmDeleteProfile: () => void;
		onCancelRename: () => void;
		onConfirmRename: () => void;
		onCancelDeleteMod: () => void;
		onConfirmDeleteMod: () => void;
	}
</script>

<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
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
		renameDialogOpen = $bindable(),
		deleteModDialogOpen = $bindable(),
		newProfileName = $bindable(),
		renameError,
		renamePending,
		onNewProfileNameInput,
		onCancelDeleteProfile,
		onConfirmDeleteProfile,
		onCancelRename,
		onConfirmRename,
		onCancelDeleteMod,
		onConfirmDeleteMod
	}: ProfileDialogsProps = $props();
</script>

<AlertDialog bind:open={deleteDialogOpen}>
	<AlertDialogContent>
		<AlertDialogHeader>
			<AlertDialogTitle>Delete Profile?</AlertDialogTitle>
			<AlertDialogDescription>
				Are you sure you want to delete <strong>{profile?.name}</strong>? This action cannot be
				undone and will delete all files associated with this profile.
			</AlertDialogDescription>
		</AlertDialogHeader>
		<AlertDialogFooter>
			<AlertDialogCancel onclick={onCancelDeleteProfile}>Cancel</AlertDialogCancel>
			<AlertDialogAction
				onclick={onConfirmDeleteProfile}
				class="bg-destructive text-destructive-foreground hover:bg-destructive/90"
			>
				Delete Profile
			</AlertDialogAction>
		</AlertDialogFooter>
	</AlertDialogContent>
</AlertDialog>

<Dialog.Root bind:open={renameDialogOpen}>
	<Dialog.Content>
		<Dialog.Header>
			<Dialog.Title>Rename Profile</Dialog.Title>
			<Dialog.Description>Enter a new name for this profile.</Dialog.Description>
		</Dialog.Header>

		<div class="space-y-4 py-4">
			<Input
				bind:value={newProfileName}
				oninput={onNewProfileNameInput}
				placeholder="Profile name"
				disabled={renamePending}
			/>
			{#if renameError}
				<p class="text-sm text-destructive">{renameError}</p>
			{/if}
		</div>

		<Dialog.Footer>
			<Button variant="outline" onclick={onCancelRename}>Cancel</Button>
			<Button onclick={onConfirmRename} disabled={renamePending || !newProfileName.trim()}>
				{#if renamePending}
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

<AlertDialog bind:open={deleteModDialogOpen}>
	<AlertDialogContent>
		<AlertDialogHeader>
			<AlertDialogTitle>Remove Mod?</AlertDialogTitle>
			<AlertDialogDescription>
				Are you sure you want to remove this mod from the profile? The mod file will be deleted.
			</AlertDialogDescription>
		</AlertDialogHeader>
		<AlertDialogFooter>
			<AlertDialogCancel onclick={onCancelDeleteMod}>Cancel</AlertDialogCancel>
			<AlertDialogAction
				onclick={onConfirmDeleteMod}
				class="bg-destructive text-destructive-foreground hover:bg-destructive/90"
			>
				Remove Mod
			</AlertDialogAction>
		</AlertDialogFooter>
	</AlertDialogContent>
</AlertDialog>
