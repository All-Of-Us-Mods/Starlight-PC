export const settingsQueryKey = ["settings"] as const;
export const settingsCacheExistsQueryKey = (architecture: "x86" | "x64") =>
  ["settings", "bepinex-cache-exists", architecture] as const;
