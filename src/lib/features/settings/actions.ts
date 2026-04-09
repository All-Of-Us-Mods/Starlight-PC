import type { QueryClient } from "@tanstack/svelte-query";
import type { AppSettings } from "./schema";
import { settingsQueryKey } from "./settings-keys";
import { rustInvoke } from "$lib/infra/rust/invoke";
import {
  autoDetectBepInExArchitecture,
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
  autoDetectBepInExArchitecture: (queryClient: QueryClient) => ({
    mutationFn: async (gamePath: string) => {
      const updatedUrl = await autoDetectBepInExArchitecture(gamePath);
      if (!updatedUrl) return null;
      const updates = updatedUrl.includes("win-x64-")
        ? { bepinex_url_x64: updatedUrl }
        : { bepinex_url_x86: updatedUrl };
      await rustInvoke("core_update_settings", {
        updates,
      });
      return updatedUrl;
    },
    onSuccess: (updatedUrl: string | null) => {
      if (!updatedUrl) return;
      const updates = updatedUrl.includes("win-x64-")
        ? { bepinex_url_x64: updatedUrl }
        : { bepinex_url_x86: updatedUrl };
      queryClient.setQueryData<AppSettings | undefined>(settingsQueryKey, (current) =>
        current
          ? {
              ...current,
              ...updates,
            }
          : current,
      );
    },
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
