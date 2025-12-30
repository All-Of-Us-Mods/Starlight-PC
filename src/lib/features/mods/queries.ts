import { queryOptions } from '@tanstack/svelte-query';
import { type } from 'arktype';
import { apiFetch } from '$lib/api/client';
import { ModResponse, ModInfoResponse, ModVersion } from './schema';

// Pre-create validators (avoid recreating on every call)
const ModArrayValidator = type(ModResponse.array());

export const modQueries = {
	latest: (limit = 20, offset = 0) =>
		queryOptions({
			queryKey: ['mods', 'list', { limit, offset }] as const,
			queryFn: () => apiFetch('/api/v2/mods', ModArrayValidator)
		}),

	explore: (search: string, limit: number, offset: number, sort: string = 'trending') => {
		const q = search.trim();
		const params = `limit=${limit}&offset=${offset}`;

		return queryOptions({
			queryKey: ['mods', 'explore', q, limit, offset, sort] as const,
			gcTime: 1000 * 60 * 5,
			queryFn: () => {
				if (q) {
					return apiFetch(
						`/api/v2/mods/search?q=${encodeURIComponent(q)}&${params}`,
						ModArrayValidator
					);
				}
				switch (sort) {
					case 'trending':
						return apiFetch(`/api/v2/mods/trending?${params}`, ModArrayValidator);
					default:
						return apiFetch(`/api/v2/mods?${params}`, ModArrayValidator);
				}
			}
		});
	},

	total: () =>
		queryOptions({
			queryKey: ['mods', 'total'] as const,
			queryFn: () => apiFetch('/api/v2/mods/total', type('number'))
		}),

	trending: () =>
		queryOptions({
			queryKey: ['mods', 'trending'] as const,
			queryFn: () => apiFetch('/api/v2/mods/trending', ModArrayValidator)
		}),

	info: (id: string) =>
		queryOptions({
			queryKey: ['mods', 'info', id] as const,
			queryFn: () => apiFetch(`/api/v2/mods/${id}/info`, ModInfoResponse),
			enabled: !!id
		}),

	byId: (id: string) =>
		queryOptions({
			queryKey: ['mods', 'by-id', id] as const,
			queryFn: () => apiFetch(`/api/v2/mods/${id}`, ModResponse),
			enabled: !!id
		}),

	versions: (modId: string) =>
		queryOptions({
			queryKey: ['mods', 'versions', modId] as const,
			queryFn: () => apiFetch(`/api/v2/mods/${modId}/versions`, type(ModVersion.array()))
		})
};
