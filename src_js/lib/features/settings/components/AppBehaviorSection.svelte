<script lang="ts" module>
	export interface AppBehaviorSectionProps {
		localCloseOnLaunch: boolean;
		localAllowMultiInstanceLaunch: boolean;
	}
</script>

<script lang="ts">
	import { Label } from '$lib/components/ui/label';
	import { Switch } from '$lib/components/ui/switch';
	import SettingsSection from './SettingsSection.svelte';

	let {
		localCloseOnLaunch = $bindable(),
		localAllowMultiInstanceLaunch = $bindable()
	}: AppBehaviorSectionProps = $props();
</script>

<SettingsSection title="App Behavior">
	<div class="flex items-center justify-between">
		<div class="space-y-0.5">
			<Label for="close-on-launch">Close on Launch</Label>
			<p class="text-sm text-muted-foreground">Automatically close launcher after launching game</p>
		</div>
		<Switch id="close-on-launch" bind:checked={localCloseOnLaunch} />
	</div>
	<div class="mt-4 flex items-center justify-between">
		<div class="space-y-0.5">
			<Label for="allow-multi-instance-launch">Allow Multiple Instances</Label>
			<p class="text-sm text-muted-foreground">
				Allow launching additional game instances while one is already running
			</p>
		</div>
		<Switch id="allow-multi-instance-launch" bind:checked={localAllowMultiInstanceLaunch} />
	</div>
</SettingsSection>
