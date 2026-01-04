<script lang="ts">
	import { Library, Play, Ghost } from '@lucide/svelte';
	import ProfileCard from '$lib/features/profiles/components/ProfileCard.svelte';
	import CreateProfileDialog from '$lib/features/profiles/components/CreateProfileDialog.svelte';
	import {
		AlertDialog,
		AlertDialogAction,
		AlertDialogCancel,
		AlertDialogContent,
		AlertDialogDescription,
		AlertDialogFooter,
		AlertDialogHeader,
		AlertDialogTitle
	} from '$lib/components/ui/alert-dialog';
	import { Skeleton } from '$lib/components/ui/skeleton';
	import { createQuery, useQueryClient } from '@tanstack/svelte-query';
	import { profileQueries } from '$lib/features/profiles/queries';
	import { settingsQueries } from '$lib/features/settings/queries';
	import { launchService } from '$lib/features/profiles/launch-service';
	import { profileService } from '$lib/features/profiles/profile-service';
	import type { Profile } from '$lib/features/profiles/schema';
	import { showError, showSuccess } from '$lib/utils/toast';

	const queryClient = useQueryClient();
	const profilesQuery = createQuery(() => profileQueries.all());
	const profiles = $derived((profilesQuery.data ?? []) as Profile[]);
	const settingsQuery = createQuery(() => settingsQueries.get());
	const settings = $derived(settingsQuery.data);

	let deleteDialogOpen = $state(false);
	let profileToDelete = $state<Profile | null>(null);
	let isLaunchingVanilla = $state(false);

	async function handleLaunchVanilla() {
		isLaunchingVanilla = true;
		try {
			await launchService.launchVanilla();
		} catch (e) {
			showError(e);
		} finally {
			isLaunchingVanilla = false;
		}
	}

	async function handleLaunchProfile(profile: Profile) {
		const previousProfiles = queryClient.getQueryData<Profile[]>(['profiles']);

		queryClient.setQueryData(['profiles'], (old = []) =>
			(old as Profile[]).map((p) =>
				p.id === profile.id ? { ...p, last_launched_at: Date.now() } : p
			)
		);

		try {
			await launchService.launchProfile(profile);
			queryClient.invalidateQueries({ queryKey: ['profiles'] });
			queryClient.invalidateQueries({ queryKey: ['profiles', 'active'] });
		} catch (e) {
			queryClient.setQueryData(['profiles'], previousProfiles);
			showError(e);
		}
	}

	function confirmDeleteProfile(profileId: string) {
		const profile = profiles.find((p) => p.id === profileId);
		if (profile) {
			profileToDelete = profile;
			deleteDialogOpen = true;
		}
	}

	async function handleDeleteProfile() {
		if (!profileToDelete) return;

		const profileId = profileToDelete.id;
		const profileName = profileToDelete.name;
		deleteDialogOpen = false;

		const previousProfiles = queryClient.getQueryData<Profile[]>(['profiles']);

		queryClient.setQueryData(['profiles'], (old = []) =>
			(old as Profile[]).filter((p) => p.id !== profileId)
		);

		try {
			await profileService.deleteProfile(profileId);
			showSuccess(`Profile "${profileName}" deleted`);
		} catch (e) {
			queryClient.setQueryData(['profiles'], previousProfiles);
			showError(e);
		} finally {
			profileToDelete = null;
		}
	}

	function cancelDelete() {
		deleteDialogOpen = false;
		profileToDelete = null;
	}
</script>

<div class="px-10 py-8">
	<div class="mb-6 flex items-center justify-between gap-3">
		<div class="flex items-center gap-3">
			<div
				class="flex h-12 w-12 items-center justify-center rounded-xl bg-primary/10 ring-1 ring-primary/20"
			>
				<Library class="h-6 w-6 text-primary" />
			</div>
			<div class="space-y-0.5">
				<h1 class="text-4xl font-black tracking-tight">Library</h1>
				<p class="text-sm text-muted-foreground">Manage your profiles and launch the game.</p>
			</div>
		</div>
		<CreateProfileDialog />
	</div>

	<div class="mb-6">
		<h2 class="mb-3 text-lg font-semibold">Quick Actions</h2>
		<div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
			<button
				onclick={handleLaunchVanilla}
				disabled={isLaunchingVanilla}
				class="flex items-center gap-3 rounded-lg border border-border bg-muted/20 p-4 text-left transition-colors hover:bg-accent/50 disabled:cursor-not-allowed disabled:opacity-50"
			>
				<div class="flex h-10 w-10 shrink-0 items-center justify-center rounded-full bg-primary/10">
					{#if isLaunchingVanilla}
						<div
							class="h-5 w-5 animate-spin rounded-full border-2 border-primary border-t-transparent"
						></div>
					{:else}
						<Ghost class="h-5 w-5 text-primary" />
					{/if}
				</div>
				<div class="flex min-w-0 flex-1 flex-col">
					<div class="flex items-center gap-2">
						<div class="font-semibold">
							{isLaunchingVanilla ? 'Launching...' : 'Launch Vanilla'}
						</div>
						{#if settings?.game_platform}
							<span
								class="rounded-full px-2 py-0.5 text-xs font-medium {settings.game_platform ===
								'epic'
									? 'bg-purple-500/20 text-purple-300'
									: 'bg-blue-500/20 text-blue-300'}"
							>
								{settings.game_platform === 'epic' ? 'Epic Games' : 'Steam'}
							</span>
						{/if}
					</div>
					<div class="truncate text-sm text-muted-foreground">Play without any mods</div>
				</div>
				{#if !isLaunchingVanilla}
					<Play class="ml-2 h-5 w-5 shrink-0" />
				{/if}
			</button>
		</div>
	</div>

	<div>
		<h2 class="mb-3 text-lg font-semibold">Profiles</h2>
		{#if profilesQuery.isPending}
			<div class="grid grid-cols-1 gap-4 sm:grid-cols-2 xl:grid-cols-3">
				{#each [1, 2, 3] as i (i)}
					<div class="space-y-3 rounded-lg border border-border p-4">
						<Skeleton class="h-6 w-1/2" />
						<div class="flex gap-4">
							<Skeleton class="h-4 w-16" />
							<Skeleton class="h-4 w-20" />
						</div>
						<div class="flex justify-end gap-2">
							<Skeleton class="h-9 w-20" />
							<Skeleton class="h-9 w-8" />
						</div>
					</div>
				{/each}
			</div>
		{:else if profiles.length === 0}
			<div class="rounded-lg border border-dashed border-border p-12 text-center">
				<Library class="mx-auto mb-3 h-12 w-12 text-muted-foreground/50" />
				<h3 class="mb-1 text-lg font-semibold">No profiles yet</h3>
				<p class="mb-4 text-sm text-muted-foreground">
					Create a profile to manage your modded installations.
				</p>
				<CreateProfileDialog />
			</div>
		{:else}
			<div class="grid grid-cols-1 gap-4 sm:grid-cols-2 xl:grid-cols-3">
				{#each profiles as profile (profile.id)}
					<ProfileCard
						{profile}
						onlaunch={() => handleLaunchProfile(profile)}
						ondelete={() => confirmDeleteProfile(profile.id)}
					/>
				{/each}
			</div>
		{/if}
	</div>
</div>

<AlertDialog bind:open={deleteDialogOpen}>
	<AlertDialogContent>
		<AlertDialogHeader>
			<AlertDialogTitle>Delete Profile?</AlertDialogTitle>
			<AlertDialogDescription>
				Are you sure you want to delete <strong>{profileToDelete?.name}</strong>? This action cannot
				be undone and will delete all files associated with this profile.
			</AlertDialogDescription>
		</AlertDialogHeader>
		<AlertDialogFooter>
			<AlertDialogCancel onclick={cancelDelete}>Cancel</AlertDialogCancel>
			<AlertDialogAction
				onclick={handleDeleteProfile}
				class="bg-destructive text-destructive-foreground hover:bg-destructive/90"
			>
				Delete Profile
			</AlertDialogAction>
		</AlertDialogFooter>
	</AlertDialogContent>
</AlertDialog>
