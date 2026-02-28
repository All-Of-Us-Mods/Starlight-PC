<script lang="ts" module>
	import type { Profile } from '$lib/features/profiles/schema';

	export interface LibraryProfilesSectionProps {
		isPending: boolean;
		profiles: Profile[];
		allowMultiInstanceLaunch: boolean;
		onCreateProfile: () => void;
		onLaunchProfile: (profile: Profile) => void;
		onDeleteProfile: (profileId: string) => void;
	}
</script>

<script lang="ts">
	import { Library, Plus } from '@lucide/svelte';
	import ProfileCard from '$lib/features/profiles/components/ProfileCard.svelte';
	import { Button } from '$lib/components/ui/button';
	import { Skeleton } from '$lib/components/ui/skeleton';

	let {
		isPending,
		profiles,
		allowMultiInstanceLaunch,
		onCreateProfile,
		onLaunchProfile,
		onDeleteProfile
	}: LibraryProfilesSectionProps = $props();
</script>

<div>
	<h2 class="mb-3 text-lg font-semibold">Profiles</h2>
	{#if isPending}
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
			<Button onclick={onCreateProfile}>
				<Plus class="mr-2 h-4 w-4" />
				Create Profile
			</Button>
		</div>
	{:else}
		<div class="grid grid-cols-1 gap-4 sm:grid-cols-2 xl:grid-cols-3">
			{#each profiles as profile (profile.id)}
				<ProfileCard
					{profile}
					{allowMultiInstanceLaunch}
					onlaunch={() => onLaunchProfile(profile)}
					ondelete={() => onDeleteProfile(profile.id)}
				/>
			{/each}
		</div>
	{/if}
</div>
