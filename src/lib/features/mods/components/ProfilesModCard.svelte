<script lang="ts">
	import * as Card from '$lib/components/ui/card';
	import { ImageOff, Trash } from '@jis3r/icons';
	import type { Mod } from '$lib/features/mods/schema';
	import Button from '$lib/components/ui/button/button.svelte';

	let { mod }: { mod: Mod } = $props();
</script>

<Card.Root class="overflow-hidden p-0 transition-colors hover:bg-accent/50">
  <div class="flex p-3 gap-3 relative">
    <div class="flex-none w-16 h-16 bg-muted flex items-center justify-center rounded-lg overflow-hidden">
      {#if mod._links.thumbnail}
        <img
          src={mod._links.thumbnail}
          alt={mod.name}
          class="h-full w-full object-cover"
        />
      {:else}
        <ImageOff class="h-10 w-10 text-muted-foreground/40" />
      {/if}
    </div>

    <div class="flex-1 min-w-0 pr-12">
      <h3 class="mb-0.5 truncate text-base font-bold" title={mod.name}>
        {mod.name}
      </h3>
      <p class="mb-1 truncate text-sm text-muted-foreground/80">
        by {mod.author}
      </p>
      <p class="line-clamp-3 text-sm text-muted-foreground">
        {mod.description}
      </p>
    </div>

    <Button
      size="sm"
      variant="destructive"
      class="absolute top-3 right-3 flex-none"
    >
      <Trash class="size-4" />
    </Button>
  </div>
</Card.Root>
