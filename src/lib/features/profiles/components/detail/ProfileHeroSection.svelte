<script lang="ts" module>
	import type { Profile } from '$lib/features/profiles/schema';

	export interface ProfileHeroSectionProps {
		profile: Profile;
		onOpenIconEditor: () => void;
		onOpenRename: () => void;
		onOpenDelete: () => void;
	}
</script>

<script lang="ts">
	import { join } from '@tauri-apps/api/path';
	import { revealItemInDir } from '@tauri-apps/plugin-opener';
	import { open as openDialog, save as saveDialog } from '@tauri-apps/plugin-dialog';
	import { createMutation, createQuery, useQueryClient } from '@tanstack/svelte-query';
	import { Button } from '$lib/components/ui/button';
	import { profileActions } from '$lib/features/profiles/actions';
	import { gameState } from '$lib/features/profiles/state/game-state.svelte';
	import { settingsQueries } from '$lib/features/settings/queries';
	import { rememberInstallTarget } from '$lib/features/mods/state/install-target.svelte';
	import { formatPlayTime } from '$lib/utils';
	import { showError, showSuccess } from '$lib/utils/toast';
	import {
		Calendar,
		Clock,
		Download,
		Folder,
		Link2,
		PencilLineIcon,
		Play,
		Square,
		Upload
	} from '@lucide/svelte';
	import { Trash2 } from '@jis3r/icons';
	import ProfileIcon from '$lib/features/profiles/components/ProfileIcon.svelte';

	let {
		profile,
		onOpenIconEditor,
		onOpenRename,
		onOpenDelete
	}: ProfileHeroSectionProps = $props();

	const queryClient = useQueryClient();
	const settingsQuery = createQuery(() => settingsQueries.get());

	const launchProfileMutation = createMutation(() => profileActions.launchProfile(queryClient));
	const stopProfileInstancesMutation = createMutation(() => profileActions.stopProfileInstances());
	const exportProfileZip = createMutation(() => profileActions.exportZip());
	const importProfileMod = createMutation(() => profileActions.importMod(queryClient));
	const createDesktopShortcut = createMutation(() =>
		profileActions.createDesktopShortcut(queryClient)
	);

	let isLaunching = $state(false);

	const runningInstanceCount = $derived(gameState.getProfileRunningInstanceCount(profile.id));
	const isRunning = $derived(runningInstanceCount > 0);
	const isStoppable = $derived(gameState.isProfileStoppable(profile.id));
	const installState = $derived(gameState.getBepInExState(profile.id));
	const allowMultiInstanceLaunch = $derived(
		(settingsQuery.data?.allow_multi_instance_launch ?? false) as boolean
	);
	const isInstalling = $derived(
		profile.bepinex_installed === false || installState?.status === 'installing'
	);
	const isDisabled = $derived(isInstalling || isRunning);
	const isLaunchDisabled = $derived(isInstalling || (isRunning && !allowMultiInstanceLaunch));
	const isStopping = $derived(stopProfileInstancesMutation.isPending);

	const totalPlayTime = $derived(
		(profile.total_play_time ?? 0) + (isRunning ? gameState.getSessionDuration(profile.id) : 0)
	);
	const totalPlayTimeLabel = $derived(formatPlayTime(totalPlayTime));
	const lastLaunched = $derived(
		profile.last_launched_at ? new Date(profile.last_launched_at).toLocaleDateString() : 'Never'
	);

	const launchLabel = $derived(
		isStoppable
			? 'Stop'
			: isRunning
				? allowMultiInstanceLaunch
					? 'Launch Another'
					: 'Running'
				: 'Launch'
	);
	const canLaunchAnother = $derived(isRunning && allowMultiInstanceLaunch);

	async function handleLaunch() {
		if (isLaunchDisabled) return;
		isLaunching = true;
		try {
			await launchProfileMutation.mutateAsync(profile);
			rememberInstallTarget(profile.id, 'launch');
		} catch (error) {
			showError(error);
		} finally {
			isLaunching = false;
		}
	}

	async function handleStop() {
		if (!isStoppable) return;
		try {
			const stoppedCount = await stopProfileInstancesMutation.mutateAsync(profile.id);
			showSuccess(
				stoppedCount === 1
					? `Stopped "${profile.name}"`
					: `Stopped ${stoppedCount} instances for "${profile.name}"`
			);
		} catch (error) {
			showError(error, 'Stop profile');
		}
	}

	async function handleOpenFolder() {
		try {
			await revealItemInDir(await join(profile.path, 'BepInEx'));
		} catch (error) {
			showError(error, 'Open folder');
		}
	}

	async function handleImportMod() {
		try {
			const selected = (await openDialog({
				title: 'Import Profile Mod',
				multiple: false,
				directory: false,
				filters: [{ name: 'DLL files', extensions: ['dll'] }]
			})) as string | string[] | null;

			const sourcePath = Array.isArray(selected) ? selected[0] : selected;

			if (!sourcePath || typeof sourcePath !== 'string') return;

			await importProfileMod.mutateAsync({ profileId: profile.id, sourcePath });
			const importedFileName = sourcePath.split(/[/\\]/).pop() || sourcePath;
			showSuccess(`Imported mod "${importedFileName}"`);
		} catch (error) {
			showError(error, 'Import mod');
		}
	}

	async function handleExportProfile() {
		try {
			const destination = await saveDialog({
				title: 'Export Profile ZIP',
				defaultPath: `${profile.name}.zip`,
				filters: [{ name: 'ZIP Archive', extensions: ['zip'] }]
			});
			if (!destination) return;

			await exportProfileZip.mutateAsync({ profileId: profile.id, destination });
			showSuccess(`Exported "${profile.name}"`);
		} catch (error) {
			showError(error, 'Export profile');
		}
	}

	async function handleCreateDesktopShortcut() {
		try {
			await createDesktopShortcut.mutateAsync(profile);
			showSuccess(`Created desktop shortcut for "${profile.name}"`);
		} catch (error) {
			showError(error, 'Create desktop shortcut');
		}
	}
</script>

<div class="mb-8 flex flex-col items-start gap-6 md:flex-row md:items-center">
	<div
		class="group relative flex h-36 w-36 shrink-0 items-center justify-center overflow-visible rounded-lg bg-muted/20 md:h-45 md:w-45 {isRunning
			? 'ring-2 ring-green-500/50'
			: ''}"
	>
		<ProfileIcon
			{profile}
			alt={`${profile.name} icon`}
			class="rounded-lg"
			fallbackClass="h-[60%] w-[60%]"
		/>
		<Button
			variant="secondary"
			size="icon-sm"
			class="pointer-events-none absolute right-2 bottom-2 rounded-full opacity-0 shadow-sm transition-opacity group-focus-within:pointer-events-auto group-focus-within:opacity-100 group-hover:pointer-events-auto group-hover:opacity-100"
			onclick={onOpenIconEditor}
			title="Edit profile icon"
		>
			<PencilLineIcon class="size-3.5" />
		</Button>
		{#if runningInstanceCount > 0}
			<span
				class="absolute -top-2 -right-2 inline-flex min-h-6 min-w-6 items-center justify-center rounded-full bg-green-500 px-1.5 text-xs font-semibold text-white shadow-sm"
			>
				{runningInstanceCount}
			</span>
		{/if}
	</div>

	<div class="flex flex-1 flex-col gap-4">
		<div class="group inline-flex items-center gap-2">
			<h1 class="text-3xl font-extrabold tracking-tight md:text-4xl">{profile.name}</h1>
			<Button
				size="icon"
				variant="ghost"
				class="pointer-events-none size-9 rounded-full opacity-0 transition-opacity group-focus-within:pointer-events-auto group-focus-within:opacity-100 group-hover:pointer-events-auto group-hover:opacity-100"
				onclick={onOpenRename}
				title="Rename profile"
			>
				<PencilLineIcon class="size-5" />
			</Button>
		</div>

		<div class="flex flex-col gap-2 text-muted-foreground">
			<div class="inline-flex items-center gap-2 text-base md:text-lg">
				<Calendar class="size-5 text-muted-foreground/70" />
				<span>Last Launched: <span class="font-medium text-foreground">{lastLaunched}</span></span>
			</div>

			<div class="inline-flex items-center gap-2 text-base md:text-lg">
				<Clock class="size-5 text-muted-foreground/70" />
				<span>Playtime: <span class="font-medium text-foreground">{totalPlayTimeLabel}</span></span>
			</div>
		</div>

		<div class="flex flex-wrap items-center gap-2.5 pt-2 sm:gap-3">
			<Button
				size="lg"
				class="gap-2"
				onclick={isStoppable ? handleStop : handleLaunch}
				disabled={(isStoppable ? isStopping : isLaunchDisabled || isLaunching) || false}
			>
				{#if isStopping}
					<div
						class="h-4 w-4 animate-spin rounded-full border-2 border-current border-t-transparent"
					></div>
					Stopping...
				{:else if isLaunching}
					<div
						class="h-4 w-4 animate-spin rounded-full border-2 border-current border-t-transparent"
					></div>
					Launching...
				{:else}
					{#if isStoppable}
						<Square class="size-5 fill-current" />
					{:else}
						<Play class="size-5 fill-current" />
					{/if}
					<span>{launchLabel}</span>
				{/if}
			</Button>

			{#if canLaunchAnother}
				<Button
					size="lg"
					variant="outline"
					class="gap-2"
					onclick={handleLaunch}
					disabled={isLaunchDisabled || isLaunching || isStopping}
				>
					{#if isLaunching}
						<div
							class="h-4 w-4 animate-spin rounded-full border-2 border-current border-t-transparent"
						></div>
						Launching...
					{:else}
						<Play class="size-5 fill-current" />
						<span>Launch Another</span>
					{/if}
				</Button>
			{/if}

			<Button size="default" variant="outline" class="gap-1.5" onclick={handleOpenFolder}>
				<Folder class="size-4" />
				<span>Open Folder</span>
			</Button>

			<Button size="default" variant="outline" class="gap-1.5" onclick={handleImportMod}>
				<Upload class="size-4" />
				<span>Import Mod</span>
			</Button>

			<Button size="default" variant="outline" class="gap-1.5" onclick={handleExportProfile}>
				<Download class="size-4" />
				<span>Export ZIP</span>
			</Button>

			<Button
				size="default"
				variant="outline"
				class="gap-1.5"
				onclick={handleCreateDesktopShortcut}
			>
				<Link2 class="size-4" />
				<span>Create Shortcut</span>
			</Button>

			<Button
				size="lg"
				variant="destructive"
				class="gap-2"
				onclick={onOpenDelete}
				disabled={isDisabled}
			>
				<Trash2 class="size-5" />
				<span>Delete</span>
			</Button>
		</div>
	</div>
</div>
