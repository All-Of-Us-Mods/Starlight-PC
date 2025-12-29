<script lang="ts">
	import * as Dialog from '$lib/components/ui/dialog';
	import { Button } from '$lib/components/ui/button';
	import { invoke } from '@tauri-apps/api/core';

	let {
		detectedPath = '',
		open = $bindable()
	}: {
		detectedPath?: string;
		open?: boolean;
	} = $props();

	let selectedPath = $state('');
	let error = $state('');

	$effect(() => {
		if (open && detectedPath) {
			selectedPath = detectedPath;
		}
	});

	async function handleAutoDetect() {
		try {
			const path = await invoke<string | null>('detect_among_us');
			if (path) {
				selectedPath = path;
			}
		} catch {
			error = 'Failed to detect path';
		}
	}

	async function handleConfirm() {
		if (!selectedPath) {
			error = 'Please select a path';
			return;
		}

		try {
			const { settingsService } = await import('../settings-service');
			await settingsService.updateSettings({ among_us_path: selectedPath });
			open = false;
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to save path';
		}
	}

	function handleSkip() {
		open = false;
	}
</script>

<Dialog.Root bind:open>
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
			<div>
				<input
					type="text"
					class="w-full rounded-md border bg-input px-3 py-2 text-sm"
					bind:value={selectedPath}
					placeholder="C:\\Program Files\\Steam\\steamapps\\common\\Among Us"
				/>
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
