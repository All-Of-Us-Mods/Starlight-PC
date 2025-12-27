<script lang="ts">
	import * as Dialog from '$lib/components/ui/dialog';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { Label } from '$lib/components/ui/label';
	import { Plus } from '@lucide/svelte';
	import { profileService } from '../profile-service';

	let open = $state(false);
	let name = $state('');
	let isCreating = $state(false);
	let error = $state('');

	async function handleCreate() {
		error = '';
		if (!name.trim()) return;

		try {
			isCreating = true;
			await profileService.createProfile(name);
			name = '';
			open = false;
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to create profile' + e.toString();
		} finally {
			isCreating = false;
		}
	}

	// Reset state when the dialog is opened/closed
	function onOpenChange(isOpen: boolean) {
		if (isOpen) {
			error = '';
		} else {
			name = '';
		}
	}
</script>

<Dialog.Root bind:open {onOpenChange}>
	<Dialog.Trigger>
		<Button>
			<Plus class="mr-2 h-4 w-4" />
			Create Profile
		</Button>
	</Dialog.Trigger>
	<Dialog.Content>
		<Dialog.Header>
			<Dialog.Title>Create New Profile</Dialog.Title>
			<Dialog.Description>
				Enter a name for your new mod profile. BepInEx will be automatically installed.
			</Dialog.Description>
		</Dialog.Header>

		<div class="space-y-4 py-4">
			<div class="space-y-2">
				<Label for="name">Profile Name</Label>
				<Input
					id="name"
					bind:value={name}
					placeholder="My Modded Profile"
					disabled={isCreating}
					aria-invalid={!!error}
				/>
				{#if error}
					<p class="text-sm font-medium text-destructive">{error}</p>
				{/if}
			</div>

			<div class="flex justify-end gap-2">
				<Dialog.Close>
					<Button variant="outline" disabled={isCreating}>Cancel</Button>
				</Dialog.Close>
				<Button onclick={handleCreate} disabled={isCreating || !name.trim()}>
					{isCreating ? 'Creating...' : 'Create Profile'}
				</Button>
			</div>
		</div>
	</Dialog.Content>
</Dialog.Root>
