import { queryOptions } from "@tanstack/svelte-query";
import { type } from "arktype";
import { rcompare, satisfies, valid } from "semver";
import { apiFetch } from "$lib/infra/http/starlight-api";
import { ModResponse, ModVersion, ModVersionInfo, type ModDependency } from "./schema";
import {
  modsByIdKey,
  modsExploreKey,
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

function resolveLatestSatisfyingVersion(
  constraints: string[],
  versions: Array<{ version: string }>,
): string | null {
  if (versions.length === 0) return null;

  const normalizedConstraints = constraints.filter((item) => !!item.trim());
  const validVersions = versions
    .map((item) => item.version)
    .filter((version): version is string => !!valid(version))
    .toSorted((a, b) => rcompare(a, b));

  const requirementList = normalizedConstraints.length > 0 ? normalizedConstraints : ["*"];
  let hasInvalidConstraint = false;

  for (const version of validVersions) {
    try {
      if (
        requirementList.every((constraint) =>
          satisfies(version, constraint, { includePrerelease: true }),
        )
      ) {
        return version;
      }
    } catch {
      // Invalid semver requirement from API; fallback to latest version.
      hasInvalidConstraint = true;
      break;
    }
  }

  if (normalizedConstraints.length > 0) {
    if (hasInvalidConstraint) {
      return validVersions[0] ?? versions[0]?.version ?? null;
    }
    return null;
  }

  return validVersions[0] ?? versions[0]?.version ?? null;
}

export function resolveDependencyVersion(
  versionConstraint: string,
  versions: Array<{ version: string }>,
): string | null {
  return resolveLatestSatisfyingVersion([versionConstraint], versions);
}

export function resolveDependencyVersionWithConstraints(
  constraints: string[],
  versions: Array<{ version: string }>,
): string | null {
  return resolveLatestSatisfyingVersion(constraints, versions);
}

export const modQueries = {
  latest: (limit = 20, offset = 0) =>
    queryOptions({
      queryKey: modsListKey(limit, offset),
      queryFn: () => apiFetch(`/api/v3/mods?limit=${limit}&offset=${offset}`, ModArrayValidator),
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
            `/api/v3/mods/search?q=${encodeURIComponent(q)}&${params}`,
            ModArrayValidator,
          );
        }
        switch (sort) {
          case "trending":
            return apiFetch(`/api/v3/mods/trending?${params}`, ModArrayValidator);
          default:
            return apiFetch(`/api/v3/mods?${params}`, ModArrayValidator);
        }
      },
      networkMode: "offlineFirst",
    });
  },

  total: () =>
    queryOptions({
      queryKey: modsTotalKey(),
      queryFn: () => apiFetch("/api/v3/mods/total", type("number")),
      networkMode: "offlineFirst",
    }),

  trending: () =>
    queryOptions({
      queryKey: modsTrendingKey(),
      queryFn: () => apiFetch("/api/v3/mods/trending", ModArrayValidator),
      networkMode: "offlineFirst",
    }),

  info: (id: string) =>
    queryOptions({
      queryKey: modsByIdKey(id),
      queryFn: () => apiFetch(`/api/v3/mods/${id}`, ModResponse),
      enabled: !!id,
      networkMode: "offlineFirst",
    }),

  versions: (modId: string) =>
    queryOptions({
      queryKey: modsVersionsKey(modId),
      queryFn: () => apiFetch(`/api/v3/mods/${modId}/versions`, type(ModVersion.array())),
      networkMode: "offlineFirst",
    }),

  versionInfo: (modId: string, version: string) =>
    queryOptions({
      queryKey: modsVersionInfoKey(modId, version),
      queryFn: () => apiFetch(`/api/v3/mods/${modId}/versions/${version}`, ModVersionInfo),
      enabled: !!modId && !!version,
      networkMode: "offlineFirst",
    }),

  resolvedDependencies: (dependencies: ModDependency[]) => {
    const queryKey = dependencies
      .map((d) => `${d.mod_id}:${d.version_constraint}:${d.type}:${d.name}`)
      .toSorted()
      .join(",");

    return queryOptions({
      queryKey: resolvedDepsKey(queryKey),
      queryFn: async () => {
        const resolved = await Promise.all(
          dependencies.map(async (dependency) => {
            const versions = await apiFetch(
              `/api/v3/mods/${dependency.mod_id}/versions`,
              ModVersionsValidator,
            );
            const resolvedVersion = resolveDependencyVersion(
              dependency.version_constraint,
              versions,
            );
            if (!resolvedVersion) return null;
            return {
              mod_id: dependency.mod_id,
              modName: dependency.name,
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
