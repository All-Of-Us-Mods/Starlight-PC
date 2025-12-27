import { queryOptions } from '@tanstack/svelte-query';
import { type } from 'arktype';
import { apiFetch } from '$lib/api/client';
import { ModResponse, ModInfoResponse } from './schema';

// Pre-create validators (avoid recreating on every call)
const ModArrayValidator = type(ModResponse.array());

// Shared config
const STALE = {
	short: 1000 * 60 * 5, // 5 min
	medium: 1000 * 60 * 15, // 15 min
	long: 1000 * 60 * 30 // 30 min
} as const;

export const modQueries = {
	latest: (limit = 20, offset = 0) =>
		queryOptions({
			queryKey: ['mods', 'list', { limit, offset }] as const,
			queryFn: () => apiFetch('/api/v2/mods', ModArrayValidator),
			staleTime: STALE.short
		}),

	explore: (search: string, limit: number, offset: number) => {
		const trimmed = search.trim();
		const params = `limit=${limit}&offset=${offset}`;

		return queryOptions({
			queryKey: ['mods', 'explore', trimmed, limit, offset] as const,
			queryFn: () =>
				apiFetch(
					trimmed
						? `/api/v2/mods/search?q=${encodeURIComponent(trimmed)}&${params}`
						: `/api/v2/mods?${params}`,
					ModArrayValidator
				),
			staleTime: STALE.short
		});
	},

	total: () =>
		queryOptions({
			queryKey: ['mods', 'total'] as const,
			queryFn: () => apiFetch('/api/v2/mods/total', type('number')),
			staleTime: STALE.long
		}),

	trending: () =>
		queryOptions({
			queryKey: ['mods', 'trending'] as const,
			queryFn: () => apiFetch('/api/v2/mods/trending', ModArrayValidator),
			staleTime: STALE.short
		}),

	info: (id: string) =>
		queryOptions({
			queryKey: ['mods', 'info', id] as const,
			queryFn: () => apiFetch(`/api/v2/mods/${id}/info`, ModInfoResponse),
			staleTime: STALE.medium,
			enabled: !!id
		}),

	byId: (id: string) =>
		queryOptions({
			queryKey: ['mods', 'by-id', id] as const,
			queryFn: () => apiFetch(`/api/v2/mods/${id}`, ModResponse),
			staleTime: STALE.long,
			enabled: !!id
		})
};
