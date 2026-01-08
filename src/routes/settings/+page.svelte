<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { Skeleton } from '$lib/components/ui/skeleton';
	import PageHeader from '$lib/components/shared/PageHeader.svelte';
	import { Save, LoaderCircle } from '@lucide/svelte';
	import { Settings } from '@jis3r/icons';
	import { createQuery, useQueryClient } from '@tanstack/svelte-query';
	import { settingsQueries } from '$lib/features/settings/queries';
	import { settingsService } from '$lib/features/settings/settings-service';
	import type { AppSettings, GamePlatform } from '$lib/features/settings/schema';
	import { showError, showSuccess } from '$lib/utils/toast';
	import { invoke } from '@tauri-apps/api/core';
	import { exists } from '@tauri-apps/plugin-fs';
	import { epicService } from '$lib/features/settings/epic-service';
	import GamePathSection from '$lib/features/settings/components/GamePathSection.svelte';
	import BepInExSection from '$lib/features/settings/components/BepInExSection.svelte';
	import AppBehaviorSection from '$lib/features/settings/components/AppBehaviorSection.svelte';
	import AboutSection from '$lib/features/settings/components/AboutSection.svelte';

	const settingsQuery = createQuery(() => settingsQueries.get());
	const settings = $derived(settingsQuery.data as AppSettings | undefined);
	const queryClient = useQueryClient();

	let isLoggedIn = $state(false);
	let isSaving = $state(false);
	let isCacheExists = $state(false);

	async function refreshEpicAuth() {
		isLoggedIn = await epicService.isLoggedIn();
	}

	async function checkCacheExists() {
		try {
			const cachePath = await settingsService.getBepInExCachePath();
			isCacheExists = await invoke<boolean>('check_bepinex_cache_exists', { cachePath });
		} catch {
			isCacheExists = false;
		}
	}

	let localAmongUsPath = $state('');
	let localBepInExUrl = $state('');
	let localCloseOnLaunch = $state(false);
	let localGamePlatform = $state<GamePlatform>('steam');
	let localCacheBepInEx = $state(false);

	$effect(() => {
		if (settings) {
			localAmongUsPath = settings.among_us_path ?? '';
			localBepInExUrl = settings.bepinex_url ?? '';
			localCloseOnLaunch = settings.close_on_launch ?? false;
			localGamePlatform = settings.game_platform ?? 'steam';
			localCacheBepInEx = settings.cache_bepinex ?? false;
			refreshEpicAuth();
			checkCacheExists();
		}
	});

	async function handleSave() {
		if (localAmongUsPath) {
			const exePath = `${localAmongUsPath}/Among Us.exe`;
			if (!(await exists(exePath))) {
				showError('Selected folder does not contain Among Us.exe');
				return;
			}
		}

		isSaving = true;
		try {
			await settingsService.updateSettings({
				among_us_path: localAmongUsPath,
				bepinex_url: localBepInExUrl,
				close_on_launch: localCloseOnLaunch,
				game_platform: localGamePlatform,
				cache_bepinex: localCacheBepInEx
			});
			await settingsService.autoDetectBepInExArchitecture(localAmongUsPath);
			queryClient.invalidateQueries({ queryKey: ['settings'] });
			showSuccess('Settings saved successfully');
		} catch (e) {
			showError(e);
		} finally {
			isSaving = false;
		}
	}
</script>

<div class="scrollbar-styled h-full overflow-y-auto px-10 py-8">
	<PageHeader
		title="Settings"
		description="Configure your Among Us path and app preferences."
		icon={Settings}
	>
		<Button onclick={handleSave} disabled={isSaving || settingsQuery.isPending} class="gap-2">
			{#if isSaving}
				<LoaderCircle class="h-4 w-4 animate-spin" />
				Saving...
			{:else}
				<Save class="h-4 w-4" />
				Save Settings
			{/if}
		</Button>
	</PageHeader>

	{#if settingsQuery.isPending}
		<div class="grid max-w-4xl gap-6 lg:grid-cols-2">
			{#each [1, 2, 3] as i (i)}
				<div
					class="space-y-4 rounded-xl border border-border/50 bg-card/30 p-6 backdrop-blur-sm"
					style="animation: pulse 2s ease-in-out infinite; animation-delay: {i * 150}ms"
				>
					<Skeleton class="h-5 w-1/3" />
					<Skeleton class="h-10 w-full" />
					<Skeleton class="h-4 w-2/3" />
				</div>
			{/each}
		</div>
	{:else}
		<div class="grid max-w-4xl gap-6 lg:grid-cols-2">
			<div class="space-y-6 lg:col-span-2">
				<GamePathSection
					bind:amongUsPath={localAmongUsPath}
					bind:gamePlatform={localGamePlatform}
					bind:isLoggedIn
					onRefreshAuth={refreshEpicAuth}
				/>
			</div>

			<BepInExSection
				bind:bepInExUrl={localBepInExUrl}
				bind:cacheBepInEx={localCacheBepInEx}
				bind:isCacheExists
			/>

			<AppBehaviorSection bind:closeOnLaunch={localCloseOnLaunch} />

			<div class="lg:col-span-2">
				<AboutSection />
			</div>
		</div>
	{/if}
</div>
