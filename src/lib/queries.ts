import { PUBLIC_API_URL } from "$env/static/public";
import { queryOptions } from "@tanstack/svelte-query";
import { z } from "zod";

// ============================================================================
// Schemas
// ============================================================================

const NewsItemSchema = z.object({
  id: z.number(),
  title: z.string(),
  author: z.string(),
  content: z.string(),
  created_at: z.number(),
  updated_at: z.number(),
});

export type NewsItem = z.infer<typeof NewsItemSchema>;

const TrendingModSchema = z.object({
  self: z.string(),
  mod_id: z.string(),
  mod_name: z.string(),
  author: z.string(),
  downloads: z.number(),
  thumbnail: z.string(),
  created_at: z.number(),
});

export type TrendingMod = z.infer<typeof TrendingModSchema>;

// ============================================================================
// Fetch Helpers
// ============================================================================

async function fetchWithValidation<T>(
  url: string,
  schema: z.ZodSchema<T>,
): Promise<T> {
  const response = await fetch(url);

  if (!response.ok) {
    throw new Error(`HTTP error: ${response.statusText}`);
  }

  const jsonData = await response.json();
  return schema.parse(jsonData);
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
          `${PUBLIC_API_URL}/api/v1/news`,
          z.array(NewsItemSchema),
        ),
      staleTime: 1000 * 60 * 5, // 5 minutes
    }),

  byId: (id: string | number) =>
    queryOptions({
      queryKey: ["news", id] as const,
      queryFn: () =>
        fetchWithValidation(
          `${PUBLIC_API_URL}/api/v1/news/${id}`,
          NewsItemSchema,
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
          `${PUBLIC_API_URL}/api/v1/mods/trending`,
          z.array(TrendingModSchema),
        ),
      staleTime: 1000 * 60 * 5, // 5 minutes
    }),
};
