import type { QueryClient } from "@tanstack/svelte-query";
import type { AppSettings } from "./schema";
import { settingsQueryKey } from "./settings-keys";
import { rustInvoke } from "$lib/infra/rust/invoke";
import {
  clearBepInExCache,
  detectAmongUsPath,
  detectGameStore,
  downloadBepInExToCache,
  openDataFolder,
} from "./services/settings-native.service";

type SettingsUpdate = Omit<Partial<AppSettings>, "xbox_app_id"> & {
  xbox_app_id?: string | null;
};

function normalizeSettingsUpdateForCache(settings: SettingsUpdate): Partial<AppSettings> {
  const { xbox_app_id, ...rest } = settings;
  if (xbox_app_id === null || xbox_app_id === undefined) return rest;
  return { ...rest, xbox_app_id };
}

export const settingsActions = {
  update: (queryClient: QueryClient) => ({
    mutationFn: (settings: SettingsUpdate) =>
      rustInvoke("core_update_settings", { updates: settings }),
    onSuccess: (updated: AppSettings, variables: SettingsUpdate) => {
      const normalizedVariables = normalizeSettingsUpdateForCache(variables);
      queryClient.setQueryData<AppSettings | undefined>(settingsQueryKey, (current) => {
        if (!current) return updated;
        return { ...current, ...normalizedVariables, ...updated };
      });
    },
  }),
  downloadBepInExToCache: () => ({
    mutationFn: (args: { url: string; architecture: "x86" | "x64" }) =>
      downloadBepInExToCache(args.url, args.architecture),
  }),
  clearBepInExCache: () => ({
    mutationFn: (architecture: "x86" | "x64") => clearBepInExCache(architecture),
  }),
  detectAmongUsPath: () => ({
    mutationFn: () => detectAmongUsPath(),
  }),
  detectGameStore: () => ({
    mutationFn: (path: string) => detectGameStore(path),
  }),
  openDataFolder: () => ({
    mutationFn: () => openDataFolder(),
  }),
};
