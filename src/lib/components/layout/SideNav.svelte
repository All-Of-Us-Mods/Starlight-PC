<script lang="ts">
	import { createQuery } from '@tanstack/svelte-query';
	import { NavButton } from '$lib/components/ui/nav-button';
	import { Settings, Compass, House, Plus } from '@jis3r/icons';
	import { Library } from '@lucide/svelte';
	import CreateProfileDialogContainer from '$lib/features/profiles/containers/CreateProfileDialogContainer.svelte';
	import ProfileIcon from '$lib/features/profiles/components/ProfileIcon.svelte';
	import { profileQueries } from '$lib/features/profiles/queries';
	import type { Profile } from '$lib/features/profiles/schema';

	let createDialogOpen = $state(false);
	const profilesQuery = createQuery(() => profileQueries.all());
	const profiles = $derived((profilesQuery.data ?? []) as Profile[]);
	const sortedProfiles = $derived(
		profiles.toSorted((a, b) => (b.last_launched_at ?? 0) - (a.last_launched_at ?? 0))
	);
	const MAX_VISIBLE_PROFILES = 3;
	const visibleProfiles = $derived(sortedProfiles.slice(0, MAX_VISIBLE_PROFILES));
	const hiddenCount = $derived(Math.max(0, sortedProfiles.length - visibleProfiles.length));
</script>

<nav class="side-nav">
	<NavButton to="/" isPrimary={(p) => p.url.pathname === '/'} tooltip="Home">
		<House class="h-6 w-6" />
	</NavButton>
	<NavButton
		to="/explore"
		isPrimary={(p) => p.url.pathname.startsWith('/explore')}
		tooltip="Explore Mods"
	>
		<Compass class="h-6 w-6" />
	</NavButton>
	<NavButton to="/library" isPrimary={(p) => p.url.pathname === '/library'} tooltip="Your Library">
		<Library class="h-6 w-6" />
	</NavButton>

	<div class="nav-divider"></div>

	<div class="nav-slot">
		<NavButton to={() => (createDialogOpen = true)} tooltip="Create New">
			<Plus class="h-6 w-6" />
		</NavButton>
	</div>

	{#if visibleProfiles.length > 0 || hiddenCount > 0}
		<div class="profile-shortcuts" aria-label="Profile Shortcuts">
			{#each visibleProfiles as profile (profile.id)}
				<NavButton
					to={`/library/${profile.id}`}
					tooltip={profile.name}
					isPrimary={(p) => p.url.pathname === `/library/${profile.id}`}
				>
					<span class="profile-icon-shell">
						<ProfileIcon {profile} class="rounded-full" fallbackClass="h-5 w-5" />
					</span>
				</NavButton>
			{/each}

			{#if hiddenCount > 0}
				<NavButton to="/library" tooltip={`Show ${hiddenCount} more profiles`}>
					<span class="profile-overflow">+{hiddenCount}</span>
				</NavButton>
			{/if}
		</div>
	{/if}

	<CreateProfileDialogContainer bind:open={createDialogOpen} />

	<div class="grow"></div>

	<div class="nav-slot">
		<NavButton
			to="/settings"
			isPrimary={(p) => p.url.pathname.startsWith('/settings')}
			tooltip="Settings"
		>
			<Settings class="h-6 w-6" />
		</NavButton>
	</div>
</nav>

<style lang="postcss">
	@reference "$lib/../app.css";

	.side-nav {
		@apply relative z-10 flex flex-col gap-2 overflow-visible bg-card/80 p-2 pt-0;
		width: var(--left-bar-width);
		grid-area: nav;
	}

	.nav-divider {
		@apply mx-auto my-2 h-px w-6 bg-accent;
	}

	.nav-slot,
	.profile-shortcuts {
		@apply flex flex-col gap-2;
	}

	.profile-icon-shell {
		@apply flex h-10 w-10 items-center justify-center overflow-hidden rounded-full;
	}

	.profile-overflow {
		@apply text-xs font-semibold tracking-tight;
	}
</style>
