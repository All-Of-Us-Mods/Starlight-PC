<script lang="ts">
  import { Button } from "$lib/components/ui/button";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { Minimize, Maximize, Maximize2, X } from "@lucide/svelte";

  // Cache window instance to avoid repeated calls
  const currentWindow = getCurrentWindow();

  let isMaximized = $state(false);

  currentWindow.isMaximized().then((value) => (isMaximized = value));

  currentWindow.onResized(async () => {
    isMaximized = await currentWindow.isMaximized();
  });

  async function handleToggleMaximize() {
    await currentWindow.toggleMaximize();
    isMaximized = await currentWindow.isMaximized();
  }
</script>

<section class="flex items-center h-full" data-tauri-drag-region-exclude>
  <Button
    variant="titlebar"
    onclick={() => currentWindow.minimize()}
    aria-label="Minimize window"
  >
    <Minimize />
  </Button>
  <Button
    variant="titlebar"
    onclick={handleToggleMaximize}
    aria-label={isMaximized ? "Restore window" : "Maximize window"}
  >
    {#if isMaximized}
      <Maximize2 />
    {:else}
      <Maximize />
    {/if}
  </Button>
  <Button
    variant="titlebar-close"
    onclick={() => currentWindow.close()}
    aria-label="Close window"
  >
    <X />
  </Button>
</section>
