<script lang="ts">
	import { page } from '$app/state';
	import { createQuery } from '@tanstack/svelte-query';

	import { profileQueries } from '$lib/features/profiles/queries';
	import { gameState } from '$lib/features/profiles/state/game-state.svelte';
	import type { Profile } from '$lib/features/profiles/schema';

	import { Button } from '$lib/components/ui/button';
	import { Skeleton } from '$lib/components/ui/skeleton';
	import { ArrowLeft, Package } from '@lucide/svelte';
	import ProfileHeroSection from '$lib/features/profiles/components/detail/ProfileHeroSection.svelte';
	import ProfileIconDialog from '$lib/features/profiles/components/detail/ProfileIconDialog.svelte';
	import ProfileDialogs from '$lib/features/profiles/components/detail/ProfileDialogs.svelte';
	import ProfileLogViewerContainer from '$lib/features/profiles/containers/ProfileLogViewerContainer.svelte';
	import ProfileModsSectionContainer from '$lib/features/profiles/containers/ProfileModsSectionContainer.svelte';

	const profileId = $derived(page.params.id ?? '');

	const profilesQuery = createQuery(() => profileQueries.all());

	const profile = $derived(
		((profilesQuery.data as Profile[] | undefined)?.find((entry) => entry.id === profileId) ??
			null) as Profile | null
	);

	let deleteDialogOpen = $state(false);
	let renameDialogOpen = $state(false);
	let iconDialogOpen = $state(false);

	const runningInstanceCount = $derived(
		profile ? gameState.getProfileRunningInstanceCount(profile.id) : 0
	);
	const isRunning = $derived(runningInstanceCount > 0);
	const installState = $derived(profile ? gameState.getBepInExState(profile.id) : null);
	const isInstalling = $derived(
		profile?.bepinex_installed === false || installState?.status === 'installing'
	);
	const isDisabled = $derived(isInstalling || isRunning);

	function openRenameDialog() {
		renameDialogOpen = true;
	}

	function openIconDialog() {
		if (!profile) return;
		iconDialogOpen = true;
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
	/>
{/if}
