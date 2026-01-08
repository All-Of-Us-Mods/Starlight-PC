<script lang="ts">
	import { Label } from '$lib/components/ui/label';
	import { Switch } from '$lib/components/ui/switch';

	let { closeOnLaunch = $bindable(false) }: { closeOnLaunch: boolean } = $props();
</script>

<div class="rounded-lg border border-border p-6">
	<h2 class="mb-4 text-lg font-semibold">App Behavior</h2>
	<div class="flex items-center justify-between">
		<div class="space-y-0.5">
			<Label for="close-on-launch">Close on Launch</Label>
			<p class="text-sm text-muted-foreground">Automatically close launcher after launching game</p>
		</div>
		<Switch id="close-on-launch" bind:checked={closeOnLaunch} />
	</div>
</div>
