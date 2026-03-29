<script lang="ts" module>
	import type { Profile } from '$lib/features/profiles/schema';

	export interface ProfileHeroSectionProps {
		profile: Profile;
		isRunning: boolean;
		isStoppable: boolean;
		runningInstanceCount: number;
		allowMultiInstanceLaunch: boolean;
		lastLaunched: string;
		totalPlayTimeLabel: string;
		isDisabled: boolean;
		isLaunchDisabled: boolean;
		isLaunching: boolean;
		isStopping: boolean;
		onLaunch: () => void | Promise<void>;
		onStop: () => void | Promise<void>;
		onOpenFolder: () => void | Promise<void>;
		onImportDll: () => void | Promise<void>;
		onExport: () => void | Promise<void>;
		onCreateDesktopShortcut: () => void | Promise<void>;
		onOpenIconEditor: () => void;
		onOpenRename: () => void;
		onOpenDelete: () => void;
	}
</script>

<script lang="ts">
	import { Button } from '$lib/components/ui/button';
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
		isRunning,
		isStoppable,
		runningInstanceCount,
		allowMultiInstanceLaunch,
		lastLaunched,
		totalPlayTimeLabel,
		isDisabled,
		isLaunchDisabled,
		isLaunching,
		isStopping,
		onLaunch,
		onStop,
		onOpenFolder,
		onImportDll,
		onExport,
		onCreateDesktopShortcut,
		onOpenIconEditor,
		onOpenRename,
		onOpenDelete
	}: ProfileHeroSectionProps = $props();

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
				onclick={isStoppable ? onStop : onLaunch}
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
					onclick={onLaunch}
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

			<Button size="default" variant="outline" class="gap-1.5" onclick={onOpenFolder}>
				<Folder class="size-4" />
				<span>Open Folder</span>
			</Button>

			<Button size="default" variant="outline" class="gap-1.5" onclick={onImportDll}>
				<Upload class="size-4" />
				<span>Import DLL</span>
			</Button>

			<Button size="default" variant="outline" class="gap-1.5" onclick={onExport}>
				<Download class="size-4" />
				<span>Export ZIP</span>
			</Button>

			<Button
				size="default"
				variant="outline"
				class="gap-1.5"
				onclick={onCreateDesktopShortcut}
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
