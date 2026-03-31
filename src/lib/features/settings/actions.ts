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
    mutationFn: (url: string) => downloadBepInExToCache(url),
  }),
  clearBepInExCache: () => ({
    mutationFn: () => clearBepInExCache(),
  }),
  autoDetectBepInExArchitecture: (queryClient: QueryClient) => ({
    mutationFn: async (gamePath: string) => {
      const updatedUrl = await autoDetectBepInExArchitecture(gamePath);
      if (!updatedUrl) return null;
      await rustInvoke("core_update_settings", {
        updates: { bepinex_url: updatedUrl },
      });
      return updatedUrl;
    },
    onSuccess: (updatedUrl: string | null) => {
      if (!updatedUrl) return;
      queryClient.setQueryData<AppSettings | undefined>(settingsQueryKey, (current) =>
        current ? { ...current, bepinex_url: updatedUrl } : current,
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
