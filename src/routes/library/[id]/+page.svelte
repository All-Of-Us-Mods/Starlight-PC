<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { resolve } from '$app/paths';
	import { join } from '@tauri-apps/api/path';
	import { revealItemInDir } from '@tauri-apps/plugin-opener';
	import { open as openDialog, save as saveDialog } from '@tauri-apps/plugin-dialog';
	import { createMutation, createQuery, useQueryClient } from '@tanstack/svelte-query';

	import { profileQueries } from '$lib/features/profiles/queries';
	import { settingsQueries } from '$lib/features/settings/queries';
	import { profileActions } from '$lib/features/profiles/actions';
	import { gameState } from '$lib/features/profiles/state/game-state.svelte';
	import { formatPlayTime } from '$lib/utils';
	import { showError, showSuccess } from '$lib/utils/toast';
	import type { Profile } from '$lib/features/profiles/schema';
	import { profileUnifiedModsKey } from '$lib/features/profiles/profile-keys';
	import { rememberInstallTarget } from '$lib/features/mods/state/install-target.svelte';

	import { Button } from '$lib/components/ui/button';
	import { Skeleton } from '$lib/components/ui/skeleton';
	import { ArrowLeft, Package } from '@lucide/svelte';
	import ProfileHeroSection from '$lib/features/profiles/components/detail/ProfileHeroSection.svelte';
	import ProfileIconDialog from '$lib/features/profiles/components/detail/ProfileIconDialog.svelte';
	import ProfileDialogs from '$lib/features/profiles/components/detail/ProfileDialogs.svelte';
	import ProfileLogViewerContainer from '$lib/features/profiles/containers/ProfileLogViewerContainer.svelte';
	import ProfileModsSectionContainer from '$lib/features/profiles/containers/ProfileModsSectionContainer.svelte';

	const queryClient = useQueryClient();
	const profileId = $derived(page.params.id ?? '');

	const profilesQuery = createQuery(() => profileQueries.all());
	const settingsQuery = createQuery(() => settingsQueries.get());

	const profile = $derived(
		((profilesQuery.data as Profile[] | undefined)?.find((entry) => entry.id === profileId) ??
			null) as Profile | null
	);

	const launchProfileMutation = createMutation(() => profileActions.launchProfile(queryClient));
	const stopProfileInstancesMutation = createMutation(() => profileActions.stopProfileInstances());
	const deleteProfile = createMutation(() => profileActions.delete(queryClient));
	const renameProfile = createMutation(() => profileActions.rename(queryClient));
	const exportProfileZip = createMutation(() => profileActions.exportZip());
	const importProfileMod = createMutation(() => profileActions.importMod(queryClient));
	const createDesktopShortcut = createMutation(() =>
		profileActions.createDesktopShortcut(queryClient)
	);

	let deleteDialogOpen = $state(false);
	let renameDialogOpen = $state(false);
	let iconDialogOpen = $state(false);
	let newProfileName = $state('');
	let isLaunching = $state(false);
	let renameError = $state('');

	const runningInstanceCount = $derived(
		profile ? gameState.getProfileRunningInstanceCount(profile.id) : 0
	);
	const isRunning = $derived(runningInstanceCount > 0);
	const isStoppable = $derived(profile ? gameState.isProfileStoppable(profile.id) : false);
	const installState = $derived(profile ? gameState.getBepInExState(profile.id) : null);
	const allowMultiInstanceLaunch = $derived(
		(settingsQuery.data?.allow_multi_instance_launch ?? false) as boolean
	);
	const isInstalling = $derived(
		profile?.bepinex_installed === false || installState?.status === 'installing'
	);
	const isDisabled = $derived(isInstalling || isRunning);
	const isLaunchDisabled = $derived(isInstalling || (isRunning && !allowMultiInstanceLaunch));

	const totalPlayTime = $derived(
		(profile?.total_play_time ?? 0) +
			(isRunning && profile ? gameState.getSessionDuration(profile.id) : 0)
	);
	const lastLaunched = $derived(
		profile?.last_launched_at ? new Date(profile.last_launched_at).toLocaleDateString() : 'Never'
	);

	async function handleLaunch() {
		if (!profile || isLaunchDisabled) return;
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
		if (!profile || !isStoppable) return;
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

	async function handleExportProfile() {
		if (!profile) return;

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

	async function handleImportMod() {
		if (!profile) return;

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

	async function handleCreateDesktopShortcut() {
		if (!profile) return;

		try {
			await createDesktopShortcut.mutateAsync(profile);
			showSuccess(`Created desktop shortcut for "${profile.name}"`);
		} catch (error) {
			showError(error, 'Create desktop shortcut');
		}
	}

	async function handleDeleteProfile() {
		if (!profile) return;
		deleteDialogOpen = false;
		try {
			await deleteProfile.mutateAsync(profile.id);
			queryClient.removeQueries({ queryKey: profileUnifiedModsKey(profile.id) });
			showSuccess(`Profile "${profile.name}" deleted`);
			goto(resolve('/library'));
		} catch (error) {
			showError(error);
		}
	}

	function openRenameDialog() {
		if (!profile) return;
		newProfileName = profile.name;
		renameError = '';
		renameDialogOpen = true;
	}

	function openIconDialog() {
		if (!profile) return;
		iconDialogOpen = true;
	}

	async function handleRenameProfile() {
		if (!profile || !newProfileName.trim()) return;
		renameError = '';
		try {
			await renameProfile.mutateAsync({ profileId: profile.id, newName: newProfileName });
			showSuccess('Profile renamed');
			renameDialogOpen = false;
		} catch (error) {
			renameError = error instanceof Error ? error.message : 'Failed to rename';
		}
	}

	async function openProfileFolder(profileEntry: Profile) {
		try {
			await revealItemInDir(await join(profileEntry.path, 'BepInEx'));
		} catch (error) {
			showError(error, 'Open folder');
		}
	}
</script>

{#if profilesQuery.isPending}
	<div class="px-10 py-8">
		<div class="mb-8 flex flex-col items-start gap-6 md:flex-row md:items-center">
			<Skeleton class="h-45 w-45 rounded-lg" />
			<div class="flex-1 space-y-4">
				<Skeleton class="h-10 w-64" />
				<Skeleton class="h-5 w-48" />
				<Skeleton class="h-5 w-40" />
				<div class="flex gap-3">
					<Skeleton class="h-10 w-28" />
					<Skeleton class="h-10 w-32" />
					<Skeleton class="h-10 w-24" />
				</div>
			</div>
		</div>
	</div>
{:else if !profile}
	<div class="flex h-full flex-col items-center justify-center gap-4 px-10 py-8">
		<Package class="h-16 w-16 text-muted-foreground/30" />
		<h2 class="text-xl font-bold">Profile not found</h2>
		<p class="text-muted-foreground">This profile may have been deleted.</p>
		<Button href="/library">
			<ArrowLeft class="mr-2 h-4 w-4" />
			Back to Library
		</Button>
	</div>
{:else}
	<div class="px-10 py-8">
		<Button variant="ghost" size="sm" class="mb-4" href="/library">
			<ArrowLeft class="mr-2 h-4 w-4" />
			Back to Library
		</Button>

		<ProfileHeroSection
			{profile}
			{isRunning}
			{isStoppable}
			{runningInstanceCount}
			{allowMultiInstanceLaunch}
			{lastLaunched}
			totalPlayTimeLabel={formatPlayTime(totalPlayTime)}
			{isDisabled}
			{isLaunchDisabled}
			{isLaunching}
			isStopping={stopProfileInstancesMutation.isPending}
			onLaunch={handleLaunch}
			onStop={handleStop}
			onOpenFolder={() => openProfileFolder(profile)}
			onImportMod={handleImportMod}
			onExport={handleExportProfile}
			onCreateDesktopShortcut={handleCreateDesktopShortcut}
			onOpenIconEditor={openIconDialog}
			onOpenRename={openRenameDialog}
			onOpenDelete={() => (deleteDialogOpen = true)}
		/>

		<hr class="my-5 border-t border-muted-foreground/20" />

		<ProfileModsSectionContainer {profile} {isDisabled} />
		<ProfileLogViewerContainer {profile} {isRunning} />
	</div>

	<ProfileIconDialog bind:open={iconDialogOpen} {profile} />

	<ProfileDialogs
		{profile}
		bind:deleteDialogOpen
		bind:renameDialogOpen
		bind:newProfileName
		{renameError}
		renamePending={renameProfile.isPending}
		onNewProfileNameInput={(event) =>
			(newProfileName = (event.currentTarget as HTMLInputElement).value)}
		onCancelDeleteProfile={() => (deleteDialogOpen = false)}
		onConfirmDeleteProfile={handleDeleteProfile}
		onCancelRename={() => (renameDialogOpen = false)}
		onConfirmRename={handleRenameProfile}
	/>
{/if}
