<script lang="ts">
	import * as Dialog from '$lib/components/ui/dialog';
	import { Button } from '$lib/components/ui/button';
	import { open as openDialog } from '@tauri-apps/plugin-dialog';
	import { exists } from '@tauri-apps/plugin-fs';
	import { createMutation, useQueryClient } from '@tanstack/svelte-query';
	import { settingsActions } from '$lib/features/settings/actions';
	import { startupState } from '$lib/features/app/state/startup.svelte';
	import { watch } from 'runed';

	const queryClient = useQueryClient();
	const updateSettingsMutation = createMutation(() => settingsActions.update(queryClient));
	const detectBepInExMutation = createMutation(() =>
		settingsActions.autoDetectBepInExArchitecture(queryClient)
	);
	const detectAmongUsPathMutation = createMutation(() => settingsActions.detectAmongUsPath());
	const detectGameStoreMutation = createMutation(() => settingsActions.detectGameStore());

	let open = $state(false);
	let selectedPath = $state('');
	let error = $state('');
	const detectedPath = $derived(startupState.detectedAmongUsPath);

	watch(
		() => startupState.amongUsPathDialogOpen,
		(isOpen) => {
			open = isOpen;
			if (!isOpen) {
				error = '';
			}
		}
	);

	watch(
		() => open,
		(isOpen) => {
			if (isOpen && detectedPath) {
				selectedPath = detectedPath;
			}
		}
	);

	async function handleAutoDetect() {
		try {
			const path = await detectAmongUsPathMutation.mutateAsync();
			if (path) {
				selectedPath = path;
			}
		} catch {
			error = 'Failed to detect path';
		}
	}

	async function detectAndSetPlatform(path: string) {
		try {
			const platform = await detectGameStoreMutation.mutateAsync(path);
			await updateSettingsMutation.mutateAsync({ game_platform: platform });
		} catch {
			// Fallback to steam if detection fails
		}
	}

	async function handleAutoSetBepinex() {
		await detectBepInExMutation.mutateAsync(selectedPath);
	}

	async function handleBrowse() {
		try {
			const selected = await openDialog({
				directory: true,
				multiple: false,
				title: 'Select Among Us Installation Folder'
			});
			if (selected) {
				selectedPath = selected;
			}
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to browse for folder';
		}
	}

	async function handleConfirm() {
		if (!selectedPath) {
			error = 'Please select a path';
			return;
		}

		try {
			const exePath = `${selectedPath}/Among Us.exe`;
			if (!(await exists(exePath))) {
				error = 'Selected folder does not contain Among Us.exe';
				return;
			}

			await updateSettingsMutation.mutateAsync({ among_us_path: selectedPath });
			await detectAndSetPlatform(selectedPath);
			await handleAutoSetBepinex();
			startupState.hideAmongUsPathDialog();
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to save path';
		}
	}

	function handleSkip() {
		startupState.hideAmongUsPathDialog();
	}

	function handleOpenChange(isOpen: boolean) {
		open = isOpen;
		if (!isOpen) {
			startupState.hideAmongUsPathDialog();
		}
	}
</script>

<Dialog.Root bind:open onOpenChange={handleOpenChange}>
	<Dialog.Content>
		<Dialog.Header>
			<Dialog.Title>Configure Among Us Path</Dialog.Title>
			<Dialog.Description>
				{#if detectedPath}
					We detected your Among Us installation. Confirm or choose a different path.
				{:else}
					We couldn't auto-detect your Among Us installation. Please select it manually.
				{/if}
			</Dialog.Description>
		</Dialog.Header>

		<div class="space-y-4 py-4">
			<div class="flex gap-2">
				<input
					type="text"
					class="flex-1 rounded-md border bg-input px-3 py-2 text-sm"
					bind:value={selectedPath}
					placeholder="C:\\Program Files\\Steam\\steamapps\\common\\Among Us"
				/>
				<Button variant="outline" onclick={handleBrowse}>Browse</Button>
			</div>

			<div class="flex gap-2">
				<Button variant="outline" onclick={handleAutoDetect}>Auto-detect</Button>
				<Button variant="ghost" onclick={handleSkip}>Skip</Button>
				<Button onclick={handleConfirm} disabled={!selectedPath}>Confirm</Button>
			</div>

			{#if error}<p class="text-sm text-destructive">{error}</p>{/if}
		</div>
	</Dialog.Content>
</Dialog.Root>
