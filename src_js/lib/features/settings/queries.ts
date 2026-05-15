import { queryOptions } from "@tanstack/svelte-query";
import { rustInvoke } from "$lib/infra/rust/invoke";
import { rustQueryOptions } from "$lib/infra/rust/query";
import { settingsCacheExistsQueryKey, settingsQueryKey } from "./settings-keys";

export const settingsQueries = {
  get: () =>
    rustQueryOptions({
      queryKey: settingsQueryKey,
      command: "core_get_settings",
    }),
  cacheExists: (architecture: "x86" | "x64") =>
    queryOptions({
      queryKey: settingsCacheExistsQueryKey(architecture),
      queryFn: async () => {
        const cachePath = await rustInvoke("core_get_bepinex_cache_path_for_arch", {
          architecture,
        });
        return rustInvoke("modding_bepinex_cache_exists", { cachePath });
      },
    }),
};
