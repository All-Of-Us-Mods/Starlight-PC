<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { Label } from '$lib/components/ui/label';
	import { Switch } from '$lib/components/ui/switch';
	import { Settings } from '@lucide/svelte';
	import { createQuery } from '@tanstack/svelte-query';
	import { settingsQueries } from '$lib/features/settings/queries';
	import { settingsService } from '$lib/features/settings/settings-service';
	import type { AppSettings } from '$lib/features/settings/schema';
	import { showToastError } from '$lib/utils/toast';

	const settingsQuery = createQuery(() => settingsQueries.get());
	const settings = $derived(settingsQuery.data as AppSettings | undefined);

	let localAmongUsPath = $state('');
	let localBepInExUrl = $state('');
	let localBepInExVersion = $state('');
	let localCloseOnLaunch = $state(false);

	$effect(() => {
		if (settings) {
			localAmongUsPath = settings.among_us_path ?? '';
			localBepInExUrl = settings.bepinex_url ?? '';
			localBepInExVersion = settings.bepinex_version ?? '';
			localCloseOnLaunch = settings.close_on_launch ?? false;
		}
	});

	async function handleSave() {
		try {
			await settingsService.updateSettings({
				among_us_path: localAmongUsPath,
				bepinex_url: localBepInExUrl,
				bepinex_version: localBepInExVersion,
				close_on_launch: localCloseOnLaunch
			});
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

	<div class="max-w-2xl space-y-6">
		<div class="rounded-lg border border-border p-6">
			<h2 class="mb-4 text-lg font-semibold">Game Configuration</h2>
			<div class="space-y-4">
				<div class="space-y-2">
					<Label for="among-us-path">Among Us Installation Path</Label>
					<Input
						id="among-us-path"
						bind:value={localAmongUsPath}
						placeholder="C:\Program Files (x86)\Steam\steamapps\common\Among Us"
					/>
					<p class="text-sm text-muted-foreground">
						The folder where Among Us is installed (contains "Among Us.exe")
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
					<Input id="bepinex-version" bind:value={localBepInExVersion} placeholder="6.0.0-be.738" />
				</div>
			</div>
		</div>

		<div class="rounded-lg border border-border p-6">
			<h2 class="mb-4 text-lg font-semibold">App Behavior</h2>
			<div class="flex items-center justify-between">
				<div class="space-y-0.5">
					<Label for="close-on-launch">Close on Launch</Label>
					<p class="text-sm text-muted-foreground">
						Automatically close the launcher after launching the game
					</p>
				</div>
				<Switch id="close-on-launch" bind:checked={localCloseOnLaunch} />
			</div>
		</div>

		<div class="flex justify-end gap-2">
			<Button onclick={handleSave}>Save Settings</Button>
		</div>
	</div>
</div>
