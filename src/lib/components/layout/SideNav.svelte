<script lang="ts">
	import { NavButton } from '$lib/components/ui/nav-button';
	import { Settings, Compass, House, Plus } from '@jis3r/icons';
	import { Library } from '@lucide/svelte';
	import CreateProfileDialog from '$lib/features/profiles/components/CreateProfileDialog.svelte';

	let createDialogOpen = $state(false);
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
	<NavButton
		to="/library"
		isPrimary={(p) => p.url.pathname.startsWith('/library')}
		tooltip="Your Library"
	>
		<Library class="h-6 w-6" />
	</NavButton>

	<div class="nav-divider"></div>

	<NavButton to={() => (createDialogOpen = true)} tooltip="Create New">
		<Plus class="h-6 w-6" />
	</NavButton>
	<CreateProfileDialog bind:open={createDialogOpen} />

	<div class="grow"></div>

	<NavButton
		to="/settings"
		isPrimary={(p) => p.url.pathname.startsWith('/settings')}
		tooltip="Settings"
	>
		<Settings class="h-6 w-6" />
	</NavButton>
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
</style>
