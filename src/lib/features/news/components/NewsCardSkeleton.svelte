<script lang="ts">
	import * as Card from '$lib/components/ui/card';
	import { ArrowRight } from '@jis3r/icons';
	import { User, Calendar } from '@lucide/svelte';
	import { Skeleton } from '$lib/components/ui/skeleton';
</script>

<Card.Root
	class="group flex h-72 cursor-pointer flex-col transition-all hover:border-primary/50 hover:shadow-lg"
>
	<Card.Header class="pb-3">
		<div class="mb-2 flex items-center justify-between text-xs text-muted-foreground">
			<div class="flex items-center gap-1.5">
				<User class="h-3.5 w-3.5" />
				<Skeleton class="h-5 w-20" />
			</div>
			<div class="flex items-center gap-1.5">
				<Calendar class="h-3.5 w-3.5" />
				<Skeleton class="h-5 w-20" />
			</div>
		</div>

		<Card.Title class="text-lg leading-snug transition-colors group-hover:text-primary">
			<Skeleton class="h-5 w-1/2" />
		</Card.Title>
	</Card.Header>

	<Card.Content class="relative line-clamp-4 flex-1 overflow-hidden px-6 py-0 text-sm">
		<div class="pointer-events-none space-y-3">
			<Skeleton class="h-4 w-full" />
			<Skeleton class="h-4 w-3/4" />
			<Skeleton class="h-4 w-full" />
			<Skeleton class="h-4 w-1/2" />
		</div>
		<div
			class="pointer-events-none absolute right-0 bottom-0 left-0 h-16 bg-linear-to-t from-card to-transparent"
		></div>
	</Card.Content>

	<Card.Footer class="pt-2">
		<div
			class="group/btn flex items-center gap-1 text-sm font-semibold text-primary hover:underline"
		>
			Read full post
			<ArrowRight />
		</div>
	</Card.Footer>
</Card.Root>
