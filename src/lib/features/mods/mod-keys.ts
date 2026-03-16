export const modsQueryKey = ['mods'] as const;
export const resolvedDepsQueryKey = ['resolved-deps'] as const;

export const modsListKey = (limit: number, offset: number) =>
	[...modsQueryKey, 'list', { limit, offset }] as const;

export const modsExploreKey = (q: string, limit: number, offset: number, sort: string) =>
	[...modsQueryKey, 'explore', q, limit, offset, sort] as const;

export const modsTotalKey = () => [...modsQueryKey, 'total'] as const;
export const modsTrendingKey = () => [...modsQueryKey, 'trending'] as const;
export const modsInfoKey = (id: string) => [...modsQueryKey, 'info', id] as const;
export const modsByIdKey = (id: string) => [...modsQueryKey, 'by-id', id] as const;
export const modsVersionsKey = (modId: string) => [...modsQueryKey, 'versions', modId] as const;
export const modsVersionInfoKey = (modId: string, version: string) =>
	[...modsQueryKey, 'versionInfo', modId, version] as const;

export const resolvedDepsKey = (hash: string) => [...resolvedDepsQueryKey, hash] as const;
