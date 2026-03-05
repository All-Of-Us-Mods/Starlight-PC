<script lang="ts">
	import * as Card from '$lib/components/ui/card';
	import { Download, Clock } from '@jis3r/icons';
	import { Skeleton } from '$lib/components/ui/skeleton';
</script>

<Card.Root class="overflow-hidden p-0 transition-colors hover:bg-accent/50">
	<div class="flex h-40">
		<div class="h-40 w-40 flex-none shrink-0">
			<Skeleton class="h-full w-full" />
		</div>

		<div class="flex min-w-0 flex-1 flex-col p-3">
			<div class="min-w-0 flex-1">
				<h3 class="mb-0.5 truncate text-base leading-tight font-bold" title="Loading">
					<Skeleton class="h-5 w-3/4" />
				</h3>
				<p class="mb-2 truncate text-sm text-muted-foreground/80">
					<Skeleton class="h-4 w-1/3" />
				</p>
				<p class="line-clamp-3 space-y-1 text-sm leading-snug text-muted-foreground">
					<Skeleton class="h-4 w-full" />
					<Skeleton class="mt-auto h-4 w-1/3" />
				</p>
			</div>

			<div class="flex items-center gap-4 pt-2 text-sm font-medium">
				<div class="flex items-center gap-1.5">
					<Download class="text-primary" />
					<Skeleton class="h-4 w-15" />
				</div>
				<div class="flex items-center gap-1.5 text-muted-foreground">
					<Clock />
					<Skeleton class="h-4 w-20" />
				</div>
			</div>
		</div>
	</div>
</Card.Root>
