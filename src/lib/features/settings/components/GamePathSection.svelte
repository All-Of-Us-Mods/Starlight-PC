<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { Label } from '$lib/components/ui/label';
	import { RefreshCw } from '@jis3r/icons';
	import type { GamePlatform } from '../schema';
	import { showError, showSuccess } from '$lib/utils/toast';
	import { invoke } from '@tauri-apps/api/core';
	import { open as openDialog } from '@tauri-apps/plugin-dialog';
	import EpicLoginDialog from './EpicLoginDialog.svelte';

	let {
		amongUsPath = $bindable(''),
		gamePlatform = $bindable<GamePlatform>('steam'),
		isLoggedIn = $bindable(false),
		onRefreshAuth
	}: {
		amongUsPath: string;
		gamePlatform: GamePlatform;
		isLoggedIn: boolean;
		onRefreshAuth: () => Promise<void>;
	} = $props();

	let isDetecting = $state(false);
	let epicLoginOpen = $state(false);

	async function handleAutoDetect() {
		isDetecting = true;
		try {
			const path = await invoke<string | null>('detect_among_us');
			if (path) {
				amongUsPath = path;
				const platform = await invoke<string>('get_game_platform', { path });
				gamePlatform = platform as GamePlatform;
				showSuccess('Among Us path detected successfully');
			} else {
				showError('Could not auto-detect Among Us installation');
			}
		} catch (e) {
			showError(e);
		} finally {
			isDetecting = false;
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
				amongUsPath = selected;
			}
		} catch (e) {
			showError(e);
		}
	}
</script>

<div class="rounded-lg border border-border p-6">
	<h2 class="mb-4 text-lg font-semibold">Game Configuration</h2>
	<div class="space-y-4">
		<div class="space-y-2">
			<Label for="among-us-path">Among Us Installation Path</Label>
			<div class="flex gap-2">
				<Input
					id="among-us-path"
					bind:value={amongUsPath}
					placeholder="C:\Program Files (x86)\Steam\steamapps\common\Among Us"
				/>
				<Button variant="outline" onclick={handleBrowse}>Browse</Button>
				<Button variant="outline" onclick={handleAutoDetect} disabled={isDetecting}>
					{#if isDetecting}
						<RefreshCw class="h-4 w-4 animate-spin" />
					{:else}
						<RefreshCw class="h-4 w-4" />
					{/if}
				</Button>
			</div>
			<p class="text-sm text-muted-foreground">
				The folder where Among Us is installed (contains "Among Us.exe")
			</p>
		</div>

		<div class="space-y-2">
			<Label for="game-platform">Game Platform</Label>
			<div class="flex gap-2">
				<Button
					variant={gamePlatform === 'steam' ? 'default' : 'outline'}
					onclick={() => (gamePlatform = 'steam')}
					class="flex-1"
				>
					Steam
				</Button>
				<Button
					variant={gamePlatform === 'epic' ? 'default' : 'outline'}
					onclick={() => (gamePlatform = 'epic')}
					class="flex-1"
				>
					Epic Games
				</Button>
			</div>
			<p class="text-sm text-muted-foreground">
				{#if gamePlatform === 'steam'}
					Steam installation
				{:else}
					Epic Games installation (requires Epic Games login)
				{/if}
			</p>
		</div>
		{#if gamePlatform === 'epic'}
			<div class="flex items-center justify-between rounded-md bg-muted/50 p-3">
				<div class="space-y-0.5">
					<p class="text-sm font-medium">Account Status</p>
					<p class="text-sm text-muted-foreground">
						{#if isLoggedIn}
							<span class="text-green-500">Logged in</span>
						{:else}
							<span class="text-orange-500">Not logged in</span>
						{/if}
					</p>
				</div>
				<Button variant="outline" size="sm" onclick={() => (epicLoginOpen = true)}>
					{#if isLoggedIn}
						Manage Account
					{:else}
						Login to Epic Games
					{/if}
				</Button>
			</div>
		{/if}
	</div>
</div>

<EpicLoginDialog bind:open={epicLoginOpen} onChange={onRefreshAuth} />
