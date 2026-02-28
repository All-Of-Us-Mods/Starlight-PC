<script lang="ts" module>
	import type { Profile } from '$lib/features/profiles/schema';

	export interface ProfileHeroSectionProps {
		profile: Profile;
		isRunning: boolean;
		runningInstanceCount: number;
		allowMultiInstanceLaunch: boolean;
		lastLaunched: string;
		totalPlayTimeLabel: string;
		isDisabled: boolean;
		isLaunchDisabled: boolean;
		isLaunching: boolean;
		onLaunch: () => void | Promise<void>;
		onOpenFolder: () => void | Promise<void>;
		onExport: () => void | Promise<void>;
		onOpenRename: () => void;
		onOpenDelete: () => void;
	}
</script>

<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { BoxIcon, Calendar, Clock, Download, Folder, PencilLineIcon, Play } from '@lucide/svelte';
	import { Trash2 } from '@jis3r/icons';

	let {
		profile,
		isRunning,
		runningInstanceCount,
		allowMultiInstanceLaunch,
		lastLaunched,
		totalPlayTimeLabel,
		isDisabled,
		isLaunchDisabled,
		isLaunching,
		onLaunch,
		onOpenFolder,
		onExport,
		onOpenRename,
		onOpenDelete
	}: ProfileHeroSectionProps = $props();

	const launchLabel = $derived(
		isRunning ? (allowMultiInstanceLaunch ? 'Launch Another' : 'Running') : 'Launch'
	);
</script>

<div class="mb-8 flex flex-col items-start gap-6 md:flex-row md:items-center">
	<div
		class="relative flex h-36 w-36 shrink-0 items-center justify-center overflow-visible rounded-lg bg-muted/20 md:h-45 md:w-45 {isRunning
			? 'ring-2 ring-green-500/50'
			: ''}"
	>
		<BoxIcon class="h-[60%] w-[60%] text-muted-foreground/50" />
		{#if runningInstanceCount > 0}
			<span
				class="absolute -top-2 -right-2 inline-flex min-h-6 min-w-6 items-center justify-center rounded-full bg-green-500 px-1.5 text-xs font-semibold text-white shadow-sm"
			>
				{runningInstanceCount}
			</span>
		{/if}
	</div>

	<div class="flex flex-1 flex-col gap-4">
		<div class="flex items-center gap-2">
			<h1 class="text-3xl font-extrabold tracking-tight md:text-4xl">{profile.name}</h1>
			<Button
				size="icon"
				variant="ghost"
				class="size-9 rounded-full"
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

		<div class="flex flex-wrap items-center gap-3 pt-2">
			<Button size="lg" class="gap-2" onclick={onLaunch} disabled={isLaunchDisabled || isLaunching}>
				{#if isLaunching}
					<div
						class="h-4 w-4 animate-spin rounded-full border-2 border-current border-t-transparent"
					></div>
					Launching...
				{:else}
					<Play class="size-5 fill-current" />
					<span>{launchLabel}</span>
				{/if}
			</Button>

			<Button size="lg" variant="outline" class="gap-2" onclick={onOpenFolder}>
				<Folder class="size-5" />
				<span>Open Folder</span>
			</Button>

			<Button size="lg" variant="outline" class="gap-2" onclick={onExport}>
				<Download class="size-5" />
				<span>Export ZIP</span>
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
