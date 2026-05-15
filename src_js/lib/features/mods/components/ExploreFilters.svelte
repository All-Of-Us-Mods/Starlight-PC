<script lang="ts" module>
	type SortKey = 'trending' | 'latest';

	export interface ExploreFiltersProps {
		searchInput: string;
		sortBy: SortKey;
		sortOptions: { value: SortKey; label: string }[];
		searchPlaceholder: string;
	}
</script>

<script lang="ts">
	import { Input } from '$lib/components/ui/input';
	import * as Select from '$lib/components/ui/select';
	import { ArrowUpDown, X } from '@lucide/svelte';
	import { Search } from '@jis3r/icons';

	let {
		searchInput = $bindable(),
		sortBy = $bindable(),
		sortOptions,
		searchPlaceholder
	}: ExploreFiltersProps = $props();
</script>

<div class="flex items-center gap-3">
	<div class="relative max-w-xs">
		<Search class="absolute top-1/2 left-3.5 -translate-y-1/2 text-muted-foreground/70" size={16} />
		<Input
			placeholder={searchPlaceholder}
			bind:value={searchInput}
			class="h-10 w-full rounded-full border-muted-foreground/10 bg-muted/50 pr-10 pl-10"
		/>
		{#if searchInput}
			<button onclick={() => (searchInput = '')} class="absolute top-1/2 right-3 -translate-y-1/2">
				<X class="size-3.5" />
			</button>
		{/if}
	</div>

	<div class="relative">
		<ArrowUpDown
			class="absolute top-1/2 left-3.5 z-10 size-3.5 -translate-y-1/2 text-muted-foreground/70"
		/>
		<Select.Root bind:value={sortBy} type="single">
			<Select.Trigger class="h-10 w-full rounded-full border-muted-foreground/10 bg-muted/50 pl-10">
				{#each sortOptions as opt (opt.value)}
					{#if opt.value === sortBy}
						{opt.label}
					{/if}
				{/each}
			</Select.Trigger>
			<Select.Content>
				{#each sortOptions as opt (opt.value)}
					<Select.Item value={opt.value}>{opt.label}</Select.Item>
				{/each}
			</Select.Content>
		</Select.Root>
	</div>
</div>
