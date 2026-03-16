import type { QueryClient } from "@tanstack/svelte-query";
import { fetch as tauriFetch } from "@tauri-apps/plugin-http";
import { warn } from "@tauri-apps/plugin-log";
import { modQueries } from "$lib/features/mods/queries";
import type { Profile } from "$lib/features/profiles/schema";
import { profileQueries } from "$lib/features/profiles/queries";
import { buildCustomIconFilePath } from "./profile-files.service";

async function getCustomIconBytes(
  queryClient: QueryClient,
  profile: Profile,
): Promise<Uint8Array | null> {
  const extension = profile.custom_icon_extension?.trim();
  if (!extension) return null;

  const iconPath = await buildCustomIconFilePath(profile.path, extension);
  const bytes = await queryClient.fetchQuery(profileQueries.binaryFile(iconPath));
  return bytes.length > 0 ? Uint8Array.from(bytes) : null;
}

async function getModIconBytes(
  queryClient: QueryClient,
  profile: Profile,
): Promise<Uint8Array | null> {
  const modId = profile.icon_mod_id?.trim();
  if (!modId) return null;

  const mod = await queryClient.fetchQuery(modQueries.byId(modId));
  const thumbnailUrl = mod._links.thumbnail;
  if (!thumbnailUrl) return null;

  const response = await tauriFetch(thumbnailUrl);
  if (!response.ok) {
    throw new Error(`Failed to download mod icon: HTTP ${response.status}`);
  }

  return new Uint8Array(await response.arrayBuffer());
}

export async function resolveProfileShortcutIconBytes(
  queryClient: QueryClient,
  profile: Profile,
): Promise<Uint8Array | null> {
  try {
    switch (profile.icon_mode) {
      case "custom":
        return await getCustomIconBytes(queryClient, profile);
      case "mod":
        return await getModIconBytes(queryClient, profile);
      default:
        return null;
    }
  } catch (error) {
    await warn(`Failed to resolve shortcut icon for profile '${profile.id}': ${error}`);
    return null;
  }
}
