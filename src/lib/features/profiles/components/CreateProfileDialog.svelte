<script lang="ts">
	import * as Dialog from '$lib/components/ui/dialog';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { Label } from '$lib/components/ui/label';
	import { Plus } from '@lucide/svelte';
	import { profileService } from '../profile-service';
	import { useQueryClient } from '@tanstack/svelte-query';
	import type { Profile } from '../schema';
	import { handleError } from '$lib/utils/error-handler';

	const queryClient = useQueryClient();

	let open = $state(false);
	let name = $state('');
	let isCreating = $state(false);
	let error = $state('');
	let pollTimer: number | null = null;

	async function waitForBepInEx(profileId: string) {
		const checkInterval = 2000;
		pollTimer = window.setInterval(() => {
			queryClient.invalidateQueries({ queryKey: ['profiles'] });

			const profiles = queryClient.getQueryData<Profile[]>(['profiles']);
			if (profiles) {
				const profile = profiles.find((p) => p.id === profileId);
				if (profile?.bepinex_installed) {
					if (pollTimer) clearInterval(pollTimer);
					pollTimer = null;
				}
			}
		}, checkInterval);
	}

	async function handleCreate() {
		error = '';
		if (!name.trim()) return;

		try {
			isCreating = true;

			const trimmed = name.trim();

			const createdProfile = await profileService.createProfile(trimmed);

			queryClient.setQueryData(['profiles'], (old: Profile[] = []) => [...old, createdProfile]);

			waitForBepInEx(createdProfile.id);

			name = '';
			open = false;
		} catch (e) {
			error = handleError(e);
		} finally {
			isCreating = false;
		}
	}

	function onOpenChange(isOpen: boolean) {
		if (isOpen) {
			error = '';
		} else {
			name = '';
			if (pollTimer) {
				clearInterval(pollTimer);
				pollTimer = null;
			}
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
				Enter a name for your new mod profile. BepInEx will be installed in the background.
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
					{#if isCreating}
						<div
							class="mr-2 h-4 w-4 animate-spin rounded-full border-2 border-current border-t-transparent"
						></div>
						Creating...
					{:else}
						Create Profile
					{/if}
				</Button>
			</div>
		</div>
	</Dialog.Content>
</Dialog.Root>
