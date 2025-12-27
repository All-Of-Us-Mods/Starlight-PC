<script lang="ts">
	import { Library, Play, Ghost } from '@lucide/svelte';
	import ProfileCard from '$lib/features/profiles/components/ProfileCard.svelte';
	import CreateProfileDialog from '$lib/features/profiles/components/CreateProfileDialog.svelte';
	import { createQuery, useQueryClient } from '@tanstack/svelte-query';
	import { profileQueries } from '$lib/features/profiles/queries';
	import { launchService } from '$lib/features/profiles/launch-service';
	import { profileService } from '$lib/features/profiles/profile-service';
	import { modInstallService } from '$lib/features/profiles/mod-install-service';
	import type { Profile } from '$lib/features/profiles/schema';
	import type { ProfileMod } from '$lib/features/profiles/schema';

	const queryClient = useQueryClient();
	const profilesQuery = createQuery(() => profileQueries.all());
	const profiles = $derived((profilesQuery.data ?? []) as Profile[]);

	async function handleLaunchVanilla() {
		try {
			await launchService.launchVanilla();
		} catch (e) {
			alert(e instanceof Error ? e.message : 'Failed to launch game');
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
		} catch (e) {
			queryClient.setQueryData(['profiles'], previousProfiles);
			alert(e instanceof Error ? e.message : 'Failed to launch profile');
		}
	}

	async function handleDeleteProfile(profileId: string) {
		if (!confirm('Delete this profile?')) return;

		const previousProfiles = queryClient.getQueryData<Profile[]>(['profiles']);

		queryClient.setQueryData(['profiles'], (old = []) =>
			(old as Profile[]).filter((p) => p.id !== profileId)
		);

		try {
			await profileService.deleteProfile(profileId);
		} catch (e) {
			queryClient.setQueryData(['profiles'], previousProfiles);
			alert(e instanceof Error ? e.message : 'Failed to delete profile');
		}
	}

	async function handleRemoveMod(profileId: string, mod: ProfileMod) {
		const previousProfiles = queryClient.getQueryData<Profile[]>(['profiles']);

		queryClient.setQueryData(['profiles'], (old = []) =>
			(old as Profile[]).map((p) =>
				p.id === profileId ? { ...p, mods: p.mods.filter((m) => m.mod_id !== mod.mod_id) } : p
			)
		);

		try {
			const profile = profiles.find((p) => p.id === profileId);
			if (!profile) return;

			const modVersionInfo = await modInstallService.getModVersionInfo(mod.mod_id, mod.version);
			await modInstallService.removeModFromProfile(modVersionInfo.file_name, profile.path);
			await profileService.removeModFromProfile(profileId, mod.mod_id);
		} catch (e) {
			queryClient.setQueryData(['profiles'], previousProfiles);
			alert(e instanceof Error ? e.message : 'Failed to remove mod from profile');
		}
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
		<div class="grid grid-cols-1 gap-4 md:grid-cols-2">
			<button
				onclick={handleLaunchVanilla}
				class="flex items-center gap-3 rounded-lg border border-border bg-muted/20 p-4 transition-colors hover:bg-accent/50"
			>
				<div class="flex h-10 w-10 items-center justify-center rounded-full bg-primary/10">
					<Ghost class="h-5 w-5 text-primary" />
				</div>
				<div class="text-left">
					<div class="font-semibold">Launch Vanilla</div>
					<div class="text-sm text-muted-foreground">Play without any mods</div>
				</div>
				<Play class="ml-auto h-5 w-5" />
			</button>
		</div>
	</div>

	<div>
		<h2 class="mb-3 text-lg font-semibold">Profiles</h2>
		{#if profiles.length === 0}
			<div class="rounded-lg border border-dashed border-border p-12 text-center">
				<Library class="mx-auto mb-3 h-12 w-12 text-muted-foreground/50" />
				<h3 class="mb-1 text-lg font-semibold">No profiles yet</h3>
				<p class="mb-4 text-sm text-muted-foreground">
					Create a profile to manage your modded installations.
				</p>
				<CreateProfileDialog />
			</div>
		{:else}
			<div class="grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3">
				{#each profiles as profile (profile.id)}
					<ProfileCard
						{profile}
						onlaunch={() => handleLaunchProfile(profile)}
						ondelete={() => handleDeleteProfile(profile.id)}
						onremove={(mod) => handleRemoveMod(profile.id, mod)}
					/>
				{/each}
			</div>
		{/if}
	</div>
</div>
