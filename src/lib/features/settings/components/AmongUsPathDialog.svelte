<script lang="ts">
	import * as Dialog from '$lib/components/ui/dialog';
	import { Button } from '$lib/components/ui/button';
	import { invoke } from '@tauri-apps/api/core';
	import { open as openDialog } from '@tauri-apps/plugin-dialog';
	import { exists } from '@tauri-apps/plugin-fs';

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

	async function handleAutoSetBepinex() {
		const crashHandlerPath = `${selectedPath}/UnityCrashHandler64.exe`;

		if ((await exists(crashHandlerPath))) {
			const { settingsService } = await import('../settings-service');
			const url = (await settingsService.getSettings()).bepinex_url;
			const updatedurl = url.replace("x86", "x64");
			await settingsService.updateSettings({ bepinex_url: updatedurl });
		}
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

			const { settingsService } = await import('../settings-service');
			await settingsService.updateSettings({ among_us_path: selectedPath });
			await handleAutoSetBepinex();
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
