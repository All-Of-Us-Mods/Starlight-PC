<script lang="ts" module>
  import type { Profile } from "$lib/features/profiles/schema";

  export interface ProfileIconProps {
    profile: Profile;
    alt?: string;
    class?: string;
    fallbackClass?: string;
  }
</script>

<script lang="ts">
  import { onDestroy } from "svelte";
  import { createQuery } from "@tanstack/svelte-query";
  import { BoxIcon } from "@lucide/svelte";
  import { modQueries } from "$lib/features/mods/queries";
  import { profileQueries } from "$lib/features/profiles/queries";
  import { buildCustomIconFilePath } from "$lib/features/profiles/services/profile-files.service";
  import { cn } from "$lib/utils";

  let {
    profile,
    alt: providedAlt,
    class: className,
    fallbackClass = "h-[60%] w-[60%]",
  }: ProfileIconProps = $props();

  const alt = $derived(providedAlt ?? `${profile.name} icon`);
  const mode = $derived(profile.icon_mode ?? "default");
  const modIconId = $derived(mode === "mod" ? (profile.icon_mod_id ?? "") : "");

  let customIconPath = $state("");
  let customIconSrc = $state<string | null>(null);
  let customIconObjectUrl: string | null = null;

  const customIconQuery = createQuery(() =>
    profileQueries.binaryFile(customIconPath),
  );
  const modIconQuery = createQuery(() => modQueries.info(modIconId));

  function setCustomIconSrc(nextUrl: string | null) {
    if (customIconObjectUrl) {
      URL.revokeObjectURL(customIconObjectUrl);
    }
    customIconObjectUrl = nextUrl;
    customIconSrc = nextUrl;
  }

  $effect(() => {
    const currentMode = mode;
    const extension = profile.custom_icon_extension ?? "";
    const profilePath = profile.path;

    if (currentMode !== "custom" || !extension) {
      customIconPath = "";
      return;
    }

    let cancelled = false;
    void buildCustomIconFilePath(profilePath, extension).then(
      (resolvedPath) => {
        if (!cancelled) {
          customIconPath = resolvedPath;
        }
      },
    );

    return () => {
      cancelled = true;
    };
  });

  $effect(() => {
    if (mode !== "custom") {
      setCustomIconSrc(null);
      return;
    }

    const bytes = customIconQuery.data;
    if (!bytes?.length) {
      setCustomIconSrc(null);
      return;
    }

    const nextUrl = URL.createObjectURL(new Blob([new Uint8Array(bytes)]));
    setCustomIconSrc(nextUrl);
  });

  onDestroy(() => {
    setCustomIconSrc(null);
  });

  const iconSrc = $derived.by(() => {
    if (mode === "custom") {
      return customIconSrc;
    }

    if (mode === "mod") {
      return modIconQuery.data?._links.thumbnail ?? null;
    }

    return null;
  });
</script>

{#if iconSrc}
  <img
    src={iconSrc}
    {alt}
    class={cn("h-full w-full object-cover", className)}
    loading="lazy"
  />
{:else}
  <BoxIcon
    class={cn("text-muted-foreground/50", fallbackClass)}
    aria-hidden="true"
  />
{/if}
