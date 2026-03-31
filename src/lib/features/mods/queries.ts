import { queryOptions } from "@tanstack/svelte-query";
import { type } from "arktype";
import { satisfies, valid } from "semver";
import { apiFetch } from "$lib/infra/http/starlight-api";
import {
  ModResponse,
  ModInfoResponse,
  ModVersion,
  ModVersionInfo,
  type ModDependency,
} from "./schema";
import {
  modsByIdKey,
  modsExploreKey,
  modsInfoKey,
  modsListKey,
  modsTotalKey,
  modsTrendingKey,
  modsVersionInfoKey,
  modsVersionsKey,
  resolvedDepsKey,
} from "./mod-keys";

// Pre-create validators (avoid recreating on every call)
const ModArrayValidator = type(ModResponse.array());
const ModVersionsValidator = type(ModVersion.array());

function resolveDependencyVersion(
  versionConstraint: string,
  versionsSortedByNewest: Array<{ version: string }>,
): string | null {
  if (versionsSortedByNewest.length === 0) return null;
  if (versionConstraint === "*") return versionsSortedByNewest[0]?.version ?? null;

  try {
    for (const item of versionsSortedByNewest) {
      if (valid(item.version) && satisfies(item.version, versionConstraint)) {
        return item.version;
      }
    }
  } catch {
    // Invalid semver requirement from API; fallback to latest version.
  }

  return versionsSortedByNewest[0]?.version ?? null;
}

export const modQueries = {
  latest: (limit = 20, offset = 0) =>
    queryOptions({
      queryKey: modsListKey(limit, offset),
      queryFn: () => apiFetch(`/api/v2/mods?limit=${limit}&offset=${offset}`, ModArrayValidator),
      networkMode: "offlineFirst",
    }),

  explore: (search: string, limit: number, offset: number, sort: string = "trending") => {
    const q = search.trim();
    const params = `limit=${limit}&offset=${offset}`;

    return queryOptions({
      queryKey: modsExploreKey(q, limit, offset, sort),
      queryFn: () => {
        if (q) {
          return apiFetch(
            `/api/v2/mods/search?q=${encodeURIComponent(q)}&${params}`,
            ModArrayValidator,
          );
        }
        switch (sort) {
          case "trending":
            return apiFetch(`/api/v2/mods/trending?${params}`, ModArrayValidator);
          default:
            return apiFetch(`/api/v2/mods?${params}`, ModArrayValidator);
        }
      },
      networkMode: "offlineFirst",
    });
  },

  total: () =>
    queryOptions({
      queryKey: modsTotalKey(),
      queryFn: () => apiFetch("/api/v2/mods/total", type("number")),
      networkMode: "offlineFirst",
    }),

  trending: () =>
    queryOptions({
      queryKey: modsTrendingKey(),
      queryFn: () => apiFetch("/api/v2/mods/trending", ModArrayValidator),
      networkMode: "offlineFirst",
    }),

  info: (id: string) =>
    queryOptions({
      queryKey: modsInfoKey(id),
      queryFn: () => apiFetch(`/api/v2/mods/${id}/info`, ModInfoResponse),
      enabled: !!id,
      networkMode: "offlineFirst",
    }),

  byId: (id: string) =>
    queryOptions({
      queryKey: modsByIdKey(id),
      queryFn: () => apiFetch(`/api/v2/mods/${id}`, ModResponse),
      enabled: !!id,
      networkMode: "offlineFirst",
    }),

  versions: (modId: string) =>
    queryOptions({
      queryKey: modsVersionsKey(modId),
      queryFn: () => apiFetch(`/api/v2/mods/${modId}/versions`, type(ModVersion.array())),
      networkMode: "offlineFirst",
    }),

  versionInfo: (modId: string, version: string) =>
    queryOptions({
      queryKey: modsVersionInfoKey(modId, version),
      queryFn: () => apiFetch(`/api/v2/mods/${modId}/versions/${version}/info`, ModVersionInfo),
      enabled: !!modId && !!version,
      networkMode: "offlineFirst",
    }),

  resolvedDependencies: (dependencies: ModDependency[]) => {
    const queryKey = dependencies
      .map((d) => `${d.mod_id}:${d.version_constraint}`)
      .toSorted()
      .join(",");

    return queryOptions({
      queryKey: resolvedDepsKey(queryKey),
      queryFn: async () => {
        const resolved = await Promise.all(
          dependencies.map(async (dependency) => {
            const [mod, versions] = await Promise.all([
              apiFetch(`/api/v2/mods/${dependency.mod_id}`, ModResponse),
              apiFetch(`/api/v2/mods/${dependency.mod_id}/versions`, ModVersionsValidator),
            ]);
            const sorted = [...versions].toSorted((a, b) => b.created_at - a.created_at);
            const resolvedVersion = resolveDependencyVersion(dependency.version_constraint, sorted);
            if (!resolvedVersion) return null;
            return {
              mod_id: dependency.mod_id,
              modName: mod.name,
              resolvedVersion,
              type: dependency.type,
            };
          }),
        );
        return resolved.filter(
          (
            item,
          ): item is {
            mod_id: string;
            modName: string;
            resolvedVersion: string;
            type: "required" | "optional" | "conflict";
          } => item !== null,
        );
      },
      enabled: dependencies.length > 0,
      networkMode: "offlineFirst",
    });
  },
};
