import { PUBLIC_API_URL } from "$env/static/public";
import { queryOptions } from "@tanstack/svelte-query";
import { type } from "arktype";

// ============================================================================
// Schemas (ArkType)
// ============================================================================

export const NewsItem = type({
  id: "number",
  title: "string",
  author: "string",
  content: "string",
  created_at: "number",
  updated_at: "number",
});

export type NewsItem = typeof NewsItem.infer;

export const TrendingMod = type({
  id: "string",
  name: "string",
  author: "string",
  description: "string",
  created_at: "number",
  updated_at: "number",
  downloads: "number",
  _links: {
    self: "string",
    info: "string",
    thumbnail: "string",
    versions: "string",
  },
});

export type TrendingMod = typeof TrendingMod.infer;

// ============================================================================
// Fetch Helpers
// ============================================================================

async function fetchWithValidation<T>(
  url: string,
  // ArkType uses a 'Type' object for validation
  validator: { assert: (data: unknown) => T },
): Promise<T> {
  const response = await fetch(url);

  if (!response.ok) {
    throw new Error(`HTTP error: ${response.statusText}`);
  }

  const jsonData = await response.json();

  // .assert() throws an ArkErrors exception if validation fails
  return validator.assert(jsonData);
}

// ============================================================================
// Query Options Factories
// ============================================================================

export const newsQueries = {
  all: () =>
    queryOptions({
      queryKey: ["news"] as const,
      queryFn: () =>
        fetchWithValidation(
          `${PUBLIC_API_URL}/api/v2/news/posts`,
          type(NewsItem.array()),
        ),
      staleTime: 1000 * 60 * 5,
    }),

  byId: (id: string | number) =>
    queryOptions({
      queryKey: ["news", id] as const,
      queryFn: () =>
        fetchWithValidation(
          `${PUBLIC_API_URL}/api/v2/news/posts/${id}`,
          NewsItem,
        ),
      staleTime: 1000 * 60 * 5,
    }),
};

export const modQueries = {
  trending: () =>
    queryOptions({
      queryKey: ["mods", "trending"] as const,
      queryFn: () =>
        fetchWithValidation(
          `${PUBLIC_API_URL}/api/v2/mods/trending`,
          type(TrendingMod.array()),
        ),
      staleTime: 1000 * 60 * 5,
    }),
};
