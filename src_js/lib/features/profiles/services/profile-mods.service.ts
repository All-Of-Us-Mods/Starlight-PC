import { rustInvoke } from "$lib/infra/rust/invoke";
import { getProfileById } from "./profile-files.service";
import type { UnifiedMod } from "../schema";

export async function removeUnifiedMod(profileId: string, mod: UnifiedMod): Promise<void> {
  const profile = await getProfileById(profileId);
  if (!profile) {
    throw new Error(`Profile '${profileId}' not found`);
  }
  await rustInvoke("profiles_delete_mod_file", {
    profilePath: profile.path,
    fileName: mod.file,
  });
  if (mod.source === "managed") {
    await rustInvoke("profiles_remove_mod", {
      profileId,
      modId: mod.mod_id,
    });
  }
}

export async function removeMissingMods(profileId: string): Promise<number> {
  const profile = await getProfileById(profileId);
  if (!profile) return 0;

  const diskFiles = await rustInvoke("profiles_get_mod_files", { profilePath: profile.path });
  const diskSet = new Set(diskFiles);
  const missingMods = profile.mods.filter((mod) => mod.file && !diskSet.has(mod.file));

  if (missingMods.length === 0) {
    return 0;
  }

  const results = await Promise.allSettled(
    missingMods.map((mod) => rustInvoke("profiles_remove_mod", { profileId, modId: mod.mod_id })),
  );

  for (const result of results) {
    if (result.status === "rejected") {
      console.error("[profiles] Failed to remove missing mod", result.reason);
    }
  }

  return missingMods.length;
}
