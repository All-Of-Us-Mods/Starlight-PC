export const newsQueryKey = ['news'] as const;
export const newsByIdKey = (id: string | number) => [...newsQueryKey, id] as const;
