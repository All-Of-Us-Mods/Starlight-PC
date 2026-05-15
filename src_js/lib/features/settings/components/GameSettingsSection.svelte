<script lang="ts" module>
	import type { GamePlatform } from '$lib/features/settings/schema';

	export interface GameSettingsSectionProps {
		localPath: string;
		localPlatform: GamePlatform;
		pathError: string;
		isDetecting: boolean;
		isLoggedIn: boolean;
		onBrowse: () => void | Promise<void>;
		onAutoDetect: () => void | Promise<void>;
		onOpenEpicLogin: () => void;
		onPathBlur: () => void | Promise<boolean>;
	}
</script>

<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { Label } from '$lib/components/ui/label';
	import { RefreshCw } from '@jis3r/icons';
	import SettingsSection from './SettingsSection.svelte';

	const PLATFORM_OPTIONS: { value: GamePlatform; label: string }[] = [
		{ value: 'steam', label: 'Steam / Itch.io' },
		{ value: 'epic', label: 'Epic Games' },
		{ value: 'xbox', label: 'Xbox / MS Store' }
	];

	let {
		localPath = $bindable(),
		localPlatform = $bindable(),
		pathError,
		isDetecting,
		isLoggedIn,
		onBrowse,
		onAutoDetect,
		onOpenEpicLogin,
		onPathBlur
	}: GameSettingsSectionProps = $props();
</script>

<SettingsSection title="Game Configuration" class="lg:col-span-2">
	<div class="space-y-4">
		<div class="space-y-2">
			<Label for="among-us-path">Among Us Installation Path</Label>
			<div class="flex gap-2">
				<Input
					id="among-us-path"
					bind:value={localPath}
					placeholder="C:\Program Files (x86)\Steam\steamapps\common\Among Us"
					onblur={onPathBlur}
					class={pathError ? 'border-destructive focus-visible:ring-destructive' : ''}
				/>
				<Button variant="outline" onclick={onBrowse}>Browse</Button>
				<Button variant="outline" onclick={onAutoDetect} disabled={isDetecting}>
					<RefreshCw class="h-4 w-4 {isDetecting ? 'animate-spin' : ''}" />
				</Button>
			</div>
			<p class="text-sm text-muted-foreground">
				The folder where Among Us is installed (contains "Among Us.exe")
			</p>
			{#if pathError}<p class="text-sm text-destructive">{pathError}</p>{/if}
		</div>

		<div class="space-y-2">
			<Label>Game Platform</Label>
			<div class="flex gap-2">
				{#each PLATFORM_OPTIONS as option (option.value)}
					<Button
						variant={localPlatform === option.value ? 'default' : 'outline'}
						onclick={() => (localPlatform = option.value)}
						class="flex-1"
					>
						{option.label}
					</Button>
				{/each}
			</div>
			<p class="text-sm text-muted-foreground">
				{localPlatform === 'steam'
					? 'Steam installation'
					: localPlatform === 'epic'
						? 'Epic Games installation (requires Epic Games login)'
						: 'Xbox / Microsoft Store installation'}
			</p>
		</div>

		{#if localPlatform === 'epic'}
			<div class="flex items-center justify-between rounded-md bg-muted/50 p-3">
				<div class="space-y-0.5">
					<p class="text-sm font-medium">Account Status</p>
					<p class="text-sm" class:text-green-500={isLoggedIn} class:text-orange-500={!isLoggedIn}>
						{isLoggedIn ? 'Logged in' : 'Not logged in'}
					</p>
				</div>
				<Button variant="outline" size="sm" onclick={onOpenEpicLogin}>
					{isLoggedIn ? 'Manage Account' : 'Login to Epic Games'}
				</Button>
			</div>
		{/if}
	</div>
</SettingsSection>
