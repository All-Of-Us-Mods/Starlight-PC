<script lang="ts">
  import {
    createMutation,
    createQuery,
    useQueryClient,
  } from "@tanstack/svelte-query";
  import { watch } from "runed";
  import { onDestroy } from "svelte";
  import { Package } from "@lucide/svelte";

  import { profileActions } from "$lib/features/profiles/actions";
  import { profileQueries } from "$lib/features/profiles/queries";
  import { modQueries } from "$lib/features/mods/queries";
  import { mapModsById } from "$lib/features/mods/components/mod-utils";
  import type { Profile } from "$lib/features/profiles/schema";
  import type { Mod } from "$lib/features/mods/schema";
  import { buildCustomIconFilePath } from "$lib/features/profiles/services/profile-files.service";
  import { showSuccess } from "$lib/utils/toast";

  import { Button } from "$lib/components/ui/button";
  import * as Dialog from "$lib/components/ui/dialog";

  interface Props {
    open: boolean;
    profile: Profile;
  }

  const uid = $props.id();
  const customIconInputId = `${uid}-custom-icon-input`;

  let { open = $bindable(), profile }: Props = $props();

  const queryClient = useQueryClient();
  const updateProfileIcon = createMutation(() =>
    profileActions.updateIcon(queryClient),
  );

  const modIds = $derived(
    Array.from(new Set(profile.mods.map((mod) => mod.mod_id) ?? [])),
  );
  const profileModsQuery = createQuery(() => ({
    queryKey: ["mods", "profile-icon-dialog-batch", profile.id, ...modIds],
    enabled: modIds.length > 0,
    queryFn: async () => {
      const results = await Promise.allSettled(
        modIds.map((id) => queryClient.fetchQuery(modQueries.info(id))),
      );
      return results
        .filter(
          (result): result is PromiseFulfilledResult<Mod> =>
            result.status === "fulfilled",
        )
        .map((result) => result.value);
    },
  }));
  const modsMap = $derived(mapModsById(profileModsQuery.data ?? []));
  const installedModsWithIcons = $derived.by(() => {
    const mods = profile.mods
      .map((profileMod) => modsMap.get(profileMod.mod_id))
      .filter((mod): mod is Mod => !!mod && !!mod._links.thumbnail)
      .filter(
        (mod, index, entries) =>
          entries.findIndex((entry) => entry.id === mod.id) === index,
      )
      .map((mod) => ({
        id: mod.id,
        name: mod.name,
        thumbnail: mod._links.thumbnail,
      }))
      .toSorted((a, b) => a.name.localeCompare(b.name));

    return mods;
  });

  let iconModeDraft = $state<"default" | "custom" | "mod">("default");
  let customIconBytesDraft = $state<Uint8Array | null>(null);
  let customIconExtensionDraft = $state("");
  let customIconDisplayPathDraft = $state("");
  let customIconPreviewSrcDraft = $state<string | null>(null);
  let iconModIdDraft = $state("");
  let iconError = $state("");
  let customIconPreviewObjectUrl: string | null = null;

  const selectedIconMod = $derived(
    installedModsWithIcons.find((mod) => mod.id === iconModIdDraft) ?? null,
  );

  function clearObjectUrl(url: string | null) {
    if (url) {
      URL.revokeObjectURL(url);
    }
  }

  function setCustomPreviewObjectUrl(nextUrl: string | null) {
    clearObjectUrl(customIconPreviewObjectUrl);
    customIconPreviewObjectUrl = nextUrl;
    customIconPreviewSrcDraft = nextUrl;
  }

  function extractCustomIconExtension(file: File): string | null {
    const nameMatch = file.name.match(/\.([a-zA-Z0-9]+)$/);
    if (nameMatch) {
      const ext = `.${nameMatch[1].toLowerCase()}`;
      if (
        [".png", ".jpg", ".jpeg", ".webp", ".gif", ".bmp", ".avif"].includes(
          ext,
        )
      ) {
        return ext;
      }
    }

    const mimeMap: Record<string, string> = {
      "image/png": ".png",
      "image/jpeg": ".jpg",
      "image/webp": ".webp",
      "image/gif": ".gif",
      "image/bmp": ".bmp",
      "image/avif": ".avif",
    };
    return mimeMap[file.type] ?? null;
  }

  async function loadLocalImageBlobUrl(
    filePath: string,
  ): Promise<string | null> {
    try {
      const bytes = await queryClient.fetchQuery(
        profileQueries.binaryFile(filePath),
      );
      if (!bytes || bytes.length === 0) return null;
      return URL.createObjectURL(new Blob([Uint8Array.from(bytes)]));
    } catch {
      return null;
    }
  }

  async function initIconDialog() {
    iconModeDraft = profile.icon_mode ?? "default";
    customIconBytesDraft = null;
    customIconExtensionDraft = profile.custom_icon_extension ?? "";
    customIconDisplayPathDraft = profile.custom_icon_extension
      ? await buildCustomIconFilePath(
          profile.path,
          profile.custom_icon_extension,
        )
      : "";
    setCustomPreviewObjectUrl(null);
    iconError = "";

    if (customIconDisplayPathDraft) {
      const preview = await loadLocalImageBlobUrl(customIconDisplayPathDraft);
      if (preview) {
        if (!open) {
          clearObjectUrl(preview);
          return;
        }
        setCustomPreviewObjectUrl(preview);
      }
    }

    iconModIdDraft = profile.icon_mod_id ?? installedModsWithIcons[0]?.id ?? "";
    if (iconModeDraft === "mod" && !iconModIdDraft) {
      iconModeDraft = "default";
    }
  }

  function setIconMode(mode: "default" | "custom" | "mod") {
    iconModeDraft = mode;
    iconError = "";
    if (
      mode === "mod" &&
      !iconModIdDraft &&
      installedModsWithIcons.length > 0
    ) {
      iconModIdDraft = installedModsWithIcons[0].id;
    }
  }

  function clearCustomIconDraft() {
    customIconBytesDraft = null;
    customIconExtensionDraft = "";
    customIconDisplayPathDraft = "";
    setCustomPreviewObjectUrl(null);
    iconError = "";
    iconModeDraft = "default";
  }

  function handleChooseCustomIcon() {
    const fileInput = document.getElementById(
      customIconInputId,
    ) as HTMLInputElement | null;
    fileInput?.click();
  }

  async function handleCustomIconInput(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    input.value = "";
    if (!file) return;

    iconError = "";
    try {
      const extension = extractCustomIconExtension(file);
      if (!extension) {
        iconError =
          "Custom icon must be a PNG, JPG, WEBP, GIF, BMP, or AVIF image";
        return;
      }
      if (file.size === 0) {
        iconError = "Selected image is empty";
        return;
      }
      if (file.size > 10 * 1024 * 1024) {
        iconError = "Image must be 10 MB or smaller";
        return;
      }

      const bytes = new Uint8Array(await file.arrayBuffer());
      const preview = URL.createObjectURL(new Blob([bytes]));

      customIconBytesDraft = bytes;
      customIconExtensionDraft = extension;
      customIconDisplayPathDraft = await buildCustomIconFilePath(
        profile.path,
        extension,
      );
      setCustomPreviewObjectUrl(preview);
      iconModeDraft = "custom";
    } catch (error) {
      iconError =
        error instanceof Error
          ? error.message
          : "Failed to read selected image";
    }
  }

  async function handleSaveProfileIcon() {
    iconError = "";
    let mutationCalled = false;

    try {
      if (iconModeDraft === "custom") {
        if (!customIconBytesDraft || !customIconExtensionDraft) {
          const hasExistingCustomIcon =
            profile.icon_mode === "custom" &&
            typeof profile.custom_icon_extension === "string" &&
            profile.custom_icon_extension.length > 0;
          if (!hasExistingCustomIcon) {
            iconError = "Choose an image for the custom icon";
            return;
          }
        } else {
          await updateProfileIcon.mutateAsync({
            profileId: profile.id,
            selection: {
              mode: "custom",
              bytes: customIconBytesDraft,
              extension: customIconExtensionDraft,
            },
          });
          mutationCalled = true;
        }
      } else if (iconModeDraft === "mod") {
        if (!iconModIdDraft) {
          iconError = "Select an installed mod icon";
          return;
        }

        await updateProfileIcon.mutateAsync({
          profileId: profile.id,
          selection: { mode: "mod", modId: iconModIdDraft },
        });
        mutationCalled = true;
      } else {
        await updateProfileIcon.mutateAsync({
          profileId: profile.id,
          selection: { mode: "default" },
        });
        mutationCalled = true;
      }

      if (!mutationCalled) {
        open = false;
        return;
      }

      open = false;
      showSuccess("Profile icon updated");
    } catch (error) {
      iconError =
        error instanceof Error
          ? error.message
          : "Failed to update profile icon";
    }
  }

  watch(
    () => open,
    (isOpen) => {
      if (isOpen) {
        void initIconDialog();
      }
    },
  );

  onDestroy(() => {
    clearObjectUrl(customIconPreviewObjectUrl);
  });
</script>

<Dialog.Root bind:open>
  <Dialog.Content>
    <Dialog.Header>
      <Dialog.Title>Edit Profile Icon</Dialog.Title>
      <Dialog.Description>
        Use the default cube, set a custom image, or use an installed mod icon.
      </Dialog.Description>
    </Dialog.Header>

    <div class="space-y-4 py-3">
      <div class="grid grid-cols-1 gap-2 sm:grid-cols-3">
        <Button
          type="button"
          variant={iconModeDraft === "default" ? "default" : "outline"}
          onclick={() => setIconMode("default")}
        >
          Default
        </Button>
        <Button
          type="button"
          variant={iconModeDraft === "custom" ? "default" : "outline"}
          onclick={() => setIconMode("custom")}
        >
          Custom Image
        </Button>
        <Button
          type="button"
          variant={iconModeDraft === "mod" ? "default" : "outline"}
          onclick={() => setIconMode("mod")}
        >
          Installed Mod
        </Button>
      </div>

      {#if iconModeDraft === "custom"}
        <input
          id={customIconInputId}
          type="file"
          accept=".png,.jpg,.jpeg,.webp,.gif,.bmp,.avif,image/png,image/jpeg,image/webp,image/gif,image/bmp,image/avif"
          class="hidden"
          onchange={handleCustomIconInput}
        />
        <div class="space-y-3">
          <div class="flex flex-wrap items-center gap-3">
            <div
              class="flex h-16 w-16 items-center justify-center overflow-hidden rounded-lg bg-muted/30"
            >
              {#if customIconPreviewSrcDraft}
                <img
                  src={customIconPreviewSrcDraft}
                  alt="Custom icon preview"
                  class="h-full w-full object-cover"
                />
              {:else}
                <Package class="h-7 w-7 text-muted-foreground/40" />
              {/if}
            </div>
            <Button
              type="button"
              variant="outline"
              onclick={handleChooseCustomIcon}
            >
              {customIconDisplayPathDraft ? "Change Image" : "Choose Image"}
            </Button>
            {#if customIconDisplayPathDraft}
              <Button
                type="button"
                variant="ghost"
                onclick={clearCustomIconDraft}>Clear</Button
              >
            {/if}
          </div>
          {#if customIconDisplayPathDraft}
            <div
              class="max-h-20 overflow-y-auto rounded-md border border-border/40 bg-muted/20 px-2 py-1.5 text-xs break-all whitespace-pre-wrap text-muted-foreground"
              title={customIconDisplayPathDraft}
            >
              {customIconDisplayPathDraft}
            </div>
          {:else}
            <p class="text-xs text-muted-foreground">
              PNG/JPG/GIF/WebP/BMP/AVIF image files.
            </p>
          {/if}
        </div>
      {:else if iconModeDraft === "mod"}
        {#if installedModsWithIcons.length === 0}
          <p class="text-sm text-muted-foreground">
            No installed managed mods with icons are available for this profile.
          </p>
        {:else}
          <div class="space-y-3">
            <label class="text-sm font-medium" for="profile-icon-mod"
              >Installed mod icon</label
            >
            <select
              id="profile-icon-mod"
              class="h-10 w-full rounded-md border bg-background px-3 text-sm"
              bind:value={iconModIdDraft}
            >
              {#each installedModsWithIcons as mod (mod.id)}
                <option value={mod.id}>{mod.name}</option>
              {/each}
            </select>
            {#if selectedIconMod}
              <div
                class="flex items-center gap-3 rounded-md border bg-muted/20 p-2"
              >
                <img
                  src={selectedIconMod.thumbnail}
                  alt={`${selectedIconMod.name} icon`}
                  class="h-12 w-12 rounded object-cover"
                />
                <p class="text-sm text-muted-foreground">
                  {selectedIconMod.name}
                </p>
              </div>
            {/if}
          </div>
        {/if}
      {/if}

      {#if iconError}
        <p class="text-sm text-destructive">{iconError}</p>
      {/if}
    </div>

    <Dialog.Footer>
      <Button variant="outline" onclick={() => (open = false)}>Cancel</Button>
      <Button
        onclick={handleSaveProfileIcon}
        disabled={updateProfileIcon.isPending ||
          (iconModeDraft === "mod" && installedModsWithIcons.length === 0)}
      >
        {#if updateProfileIcon.isPending}
          Saving...
        {:else}
          Save Icon
        {/if}
      </Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
