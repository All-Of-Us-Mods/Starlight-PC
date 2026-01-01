<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { Label } from '$lib/components/ui/label';
	import { Switch } from '$lib/components/ui/switch';
	import { Skeleton } from '$lib/components/ui/skeleton';
	import { Settings, Save, RefreshCw } from '@lucide/svelte';
	import { createQuery, useQueryClient } from '@tanstack/svelte-query';
	import { settingsQueries } from '$lib/features/settings/queries';
	import { settingsService } from '$lib/features/settings/settings-service';
	import type { AppSettings, GamePlatform } from '$lib/features/settings/schema';
	import { showToastError, showToastSuccess } from '$lib/utils/toast';
	import { invoke } from '@tauri-apps/api/core';
	import { open as openDialog } from '@tauri-apps/plugin-dialog';
	import { exists } from '@tauri-apps/plugin-fs';
	import EpicLoginDialog from '$lib/features/settings/components/EpicLoginDialog.svelte';
	import { epicService } from '$lib/features/settings/epic-service';

	const settingsQuery = createQuery(() => settingsQueries.get());
	const settings = $derived(settingsQuery.data as AppSettings | undefined);
	const queryClient = useQueryClient();

	let isLoggedIn = $state(false);
	let isSaving = $state(false);
	let isDetecting = $state(false);
	let epicLoginOpen = $state(false);

	async function refreshEpicAuth() {
		isLoggedIn = await epicService.isLoggedIn();
	}

	let localAmongUsPath = $state('');
	let localBepInExUrl = $state('');
	let localBepInExVersion = $state('');
	let localCloseOnLaunch = $state(false);
	let localGamePlatform = $state<GamePlatform>('steam');

	$effect(() => {
		if (settings) {
			localAmongUsPath = settings.among_us_path ?? '';
			localBepInExUrl = settings.bepinex_url ?? '';
			localBepInExVersion = settings.bepinex_version ?? '';
			localCloseOnLaunch = settings.close_on_launch ?? false;
			localGamePlatform = settings.game_platform ?? 'steam';
			refreshEpicAuth();
		}
	});

	async function handleSave() {
		if (localAmongUsPath) {
			const exePath = `${localAmongUsPath}/Among Us.exe`;
			if (!(await exists(exePath))) {
				showToastError('Selected folder does not contain Among Us.exe');
				return;
			}
		}

		isSaving = true;
		try {
			await settingsService.updateSettings({
				among_us_path: localAmongUsPath,
				bepinex_url: localBepInExUrl,
				bepinex_version: localBepInExVersion,
				close_on_launch: localCloseOnLaunch,
				game_platform: localGamePlatform
			});
			await handleAutoSetBepinex();
			queryClient.invalidateQueries({ queryKey: ['settings'] });
			showToastSuccess('Settings saved successfully');
		} catch (e) {
			showToastError(e);
		} finally {
			isSaving = false;
		}
	}

	async function handleAutoDetect() {
		isDetecting = true;
		try {
			const path = await invoke<string | null>('detect_among_us');
			if (path) {
				localAmongUsPath = path;
				const platform = await invoke<string>('get_game_platform', { path });
				localGamePlatform = platform as GamePlatform;
				showToastSuccess('Among Us path detected successfully');
			} else {
				showToastError('Could not auto-detect Among Us installation');
			}
		} catch (e) {
			showToastError(e);
		} finally {
			isDetecting = false;
		}
	}

	async function handleAutoSetBepinex() {
		const crashHandlerPath = `${localAmongUsPath}/UnityCrashHandler64.exe`;

		if (await exists(crashHandlerPath)) {
			const url = (await settingsService.getSettings()).bepinex_url;
			const updatedurl = url.replace('x86', 'x64');
			await settingsService.updateSettings({ bepinex_url: updatedurl });
		} else {
			const url = (await settingsService.getSettings()).bepinex_url;
			const updatedurl = url.replace('x64', 'x86');
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
				localAmongUsPath = selected;
			}
		} catch (e) {
			showToastError(e);
		}
	}
</script>

<div class="px-10 py-8">
	<div class="mb-6 flex items-center gap-3">
		<div
			class="flex h-12 w-12 items-center justify-center rounded-xl bg-primary/10 ring-1 ring-primary/20"
		>
			<Settings class="h-6 w-6 text-primary" />
		</div>
		<div class="space-y-0.5">
			<h1 class="text-4xl font-black tracking-tight">Settings</h1>
			<p class="text-sm text-muted-foreground">Configure your Among Us path and app preferences.</p>
		</div>
	</div>

	{#if settingsQuery.isPending}
		<div class="max-w-2xl space-y-6">
			<div class="space-y-4 rounded-lg border border-border p-6">
				<Skeleton class="h-6 w-1/3" />
				<Skeleton class="h-10 w-full" />
				<Skeleton class="h-4 w-2/3" />
			</div>
			<div class="space-y-4 rounded-lg border border-border p-6">
				<Skeleton class="h-6 w-1/3" />
				<Skeleton class="h-10 w-full" />
				<Skeleton class="h-10 w-full" />
			</div>
			<div class="rounded-lg border border-border p-6">
				<Skeleton class="h-6 w-1/3" />
				<div class="mt-4 flex items-center justify-between">
					<div class="space-y-2">
						<Skeleton class="h-4 w-32" />
						<Skeleton class="h-3 w-48" />
					</div>
					<Skeleton class="h-6 w-12" />
				</div>
			</div>
		</div>
	{:else}
		<div class="mx-auto max-w-2xl space-y-6">
			<div class="rounded-lg border border-border p-6">
				<h2 class="mb-4 text-lg font-semibold">Game Configuration</h2>
				<div class="space-y-4">
					<div class="space-y-2">
						<Label for="among-us-path">Among Us Installation Path</Label>
						<div class="flex gap-2">
							<Input
								id="among-us-path"
								bind:value={localAmongUsPath}
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
								variant={localGamePlatform === 'steam' ? 'default' : 'outline'}
								onclick={() => (localGamePlatform = 'steam')}
								class="flex-1"
							>
								Steam
							</Button>
							<Button
								variant={localGamePlatform === 'epic' ? 'default' : 'outline'}
								onclick={() => (localGamePlatform = 'epic')}
								class="flex-1"
							>
								Epic Games
							</Button>
						</div>
						<p class="text-sm text-muted-foreground">
							{#if localGamePlatform === 'steam'}
								Steam installation
							{:else}
								Epic Games installation (requires Epic Games login)
							{/if}
						</p>
					</div>
				</div>
			</div>

			<div class="rounded-lg border border-border p-6">
				<h2 class="mb-4 text-lg font-semibold">BepInEx Configuration</h2>
				<div class="space-y-4">
					<div class="space-y-2">
						<Label for="bepinex-url">BepInEx Download URL</Label>
						<Input
							id="bepinex-url"
							bind:value={localBepInExUrl}
							placeholder="https://builds.bepinex.dev/..."
						/>
					</div>
					<div class="space-y-2">
						<Label for="bepinex-version">BepInEx Version</Label>
						<Input
							id="bepinex-version"
							bind:value={localBepInExVersion}
							placeholder="6.0.0-be.738"
						/>
					</div>
				</div>
			</div>

			<div class="rounded-lg border border-border p-6">
				<h2 class="mb-4 text-lg font-semibold">App Behavior</h2>
				<div class="flex items-center justify-between">
					<div class="space-y-0.5">
						<Label for="close-on-launch">Close on Launch</Label>
						<p class="text-sm text-muted-foreground">
							Automatically close launcher after launching game
						</p>
					</div>
					<Switch id="close-on-launch" bind:checked={localCloseOnLaunch} />
				</div>
			</div>

			{#if localGamePlatform === 'epic'}
				<div class="rounded-lg border border-border p-6">
					<h2 class="mb-4 text-lg font-semibold">Epic Games Account</h2>
					<div class="space-y-4">
						<div class="flex items-center justify-between">
							<div class="space-y-0.5">
								<p class="font-medium">Account Status</p>
								<p class="text-sm text-muted-foreground">
									{#if isLoggedIn}
										<span class="text-green-500">Logged in</span>
									{:else}
										<span class="text-orange-500">Not logged in</span>
									{/if}
								</p>
							</div>
							<Button variant="outline" onclick={() => (epicLoginOpen = true)}>
								{#if isLoggedIn}
									Manage Account
								{:else}
									Login to Epic Games
								{/if}
							</Button>
						</div>
						{#if !isLoggedIn}
							<p class="text-sm text-muted-foreground">
								You must login to your Epic Games account to launch the Epic Games version of Among
								Us. Your session will be saved and restored automatically.
							</p>
						{/if}
					</div>
				</div>
			{/if}

			<div class="flex justify-end gap-2">
				<Button onclick={handleSave} disabled={isSaving}>
					{#if isSaving}
						Saving...
					{:else}
						<Save class="mr-2 h-4 w-4" />
						Save Settings
					{/if}
				</Button>
			</div>
		</div>
	{/if}
</div>

<EpicLoginDialog bind:open={epicLoginOpen} onChange={refreshEpicAuth} />
