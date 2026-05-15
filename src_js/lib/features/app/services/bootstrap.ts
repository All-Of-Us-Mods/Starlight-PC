import type { QueryClient } from "@tanstack/svelte-query";
import { startupState } from "../state/startup.svelte";
import { profileActions } from "$lib/features/profiles/actions";
import { profileQueries } from "$lib/features/profiles/queries";
import { settingsQueries } from "$lib/features/settings/queries";
import { parseProfileIdFromDeepLink } from "$lib/features/profiles/services/profile-deep-link.service";
import { rustInvoke } from "$lib/infra/rust/invoke";
import { hasTauriWindowInternals } from "$lib/infra/tauri/window";
import { updateState } from "$lib/features/updates/state/update-state.svelte";
import { watchProfileDirectory } from "./profile-directory-watch";
import { getCurrent, onOpenUrl } from "@tauri-apps/plugin-deep-link";
import { info, warn } from "@tauri-apps/plugin-log";
import { showError } from "$lib/utils/toast";
import { initQueryPersistence } from "$lib/infra/query/client";

async function handleDeepLinkUrls(queryClient: QueryClient, urls: string[]) {
  const profileId = urls.map(parseProfileIdFromDeepLink).find((value): value is string => !!value);
  if (!profileId) return;

  try {
    const profiles = await queryClient.fetchQuery(profileQueries.all());
    const profile = profiles.find((entry) => entry.id === profileId);
    if (!profile) {
      showError(`Profile shortcut target '${profileId}' was not found`, "Profile shortcut");
      return;
    }

    await profileActions.launchProfile(queryClient).mutationFn(profile);
  } catch (error) {
    showError(error, "Launch profile shortcut");
  }
}

export async function bootstrapApp(queryClient: QueryClient): Promise<() => void> {
  await info("Starlight frontend initialized");
  const cleanups: Array<() => void> = [];

  const [unpersist, restored] = initQueryPersistence();
  cleanups.push(unpersist);
  await restored;

  void updateState.check();

  const settings = await queryClient.fetchQuery(settingsQueries.get());
  if (!settings.among_us_path) {
    try {
      const path = await rustInvoke("platform_detect_among_us");
      startupState.showAmongUsPathDialog(path ?? "");
    } catch {
      await warn("Failed to auto-detect Among Us path");
      startupState.showAmongUsPathDialog();
    }
  }

  try {
    const profilesDir = await queryClient.fetchQuery(profileQueries.dir());
    cleanups.push(await watchProfileDirectory(queryClient, profilesDir));
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    await warn(`Failed to initialize bootstrap state: ${message}`);
  }

  if (hasTauriWindowInternals()) {
    try {
      const unlisten = await onOpenUrl((urls) => {
        void handleDeepLinkUrls(queryClient, urls);
      });
      cleanups.push(unlisten);

      const startUrls = await getCurrent();
      if (startUrls?.length) {
        void handleDeepLinkUrls(queryClient, startUrls);
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      await warn(`Failed to initialize deep-link handling: ${message}`);
    }
  }

  return () => {
    for (const cleanup of cleanups) {
      cleanup();
    }
  };
}
