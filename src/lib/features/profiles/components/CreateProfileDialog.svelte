<script lang="ts">
	import * as Dialog from '$lib/components/ui/dialog';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { Label } from '$lib/components/ui/label';
	import { profileMutations } from '../mutations';
	import { createMutation, useQueryClient } from '@tanstack/svelte-query';
	import type { Profile } from '../schema';
	import { watchDirectory } from '$lib/utils/file-watcher';
	import { join } from '@tauri-apps/api/path';

	const queryClient = useQueryClient();
	const createProfile = createMutation(() => profileMutations.create(queryClient));

	let { open = $bindable(false) }: { open?: boolean } = $props();
	let name = $state('');
	let error = $state('');
	let unwatchFn: (() => void) | null = null;

	const isCreating = $derived(createProfile.isPending);

	async function watchProfileDirectory(profileId: string, profilePath: string) {
		const pluginsPath = await join(profilePath, 'BepInEx', 'plugins');
		unwatchFn = await watchDirectory(
			pluginsPath,
			() => {
				queryClient.invalidateQueries({ queryKey: ['profiles'] });
				queryClient.invalidateQueries({ queryKey: ['disk-files', profilePath] });

				const profiles = queryClient.getQueryData<Profile[]>(['profiles']);
				if (profiles) {
					const profile = profiles.find((p) => p.id === profileId);
					if (profile?.bepinex_installed && unwatchFn) {
						unwatchFn();
						unwatchFn = null;
					}
				}
			},
			{ recursive: true }
		);
	}

	async function handleCreate() {
		error = '';
		if (!name.trim()) return;

		try {
			const trimmed = name.trim();
			const result = await createProfile.mutateAsync(trimmed);

			watchProfileDirectory(result.id, result.path);

			name = '';
			open = false;
		} catch (e) {
			error = e instanceof Error ? e.message : 'An unknown error occurred';
		}
	}

	function onOpenChange(isOpen: boolean) {
		if (isOpen) {
			error = '';
		} else {
			name = '';
			if (unwatchFn) {
				unwatchFn();
				unwatchFn = null;
			}
		}
	}

	$effect(() => {
		return () => {
			if (unwatchFn) {
				unwatchFn();
				unwatchFn = null;
			}
		};
	});
</script>

<Dialog.Root bind:open {onOpenChange}>
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
