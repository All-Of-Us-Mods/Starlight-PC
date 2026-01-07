import { queryOptions } from '@tanstack/svelte-query';
import { type } from 'arktype';
import { apiFetch } from '$lib/api/client';
import { Post } from './schema';

const PostsArray = type(Post.array());

export const newsQueries = {
	all: () =>
		queryOptions({
			queryKey: ['news'] as const,
			queryFn: () => apiFetch('/api/v2/news/posts', PostsArray)
		}),
	byId: (id: string | number) =>
		queryOptions({
			queryKey: ['news', id] as const,
			queryFn: () => apiFetch(`/api/v2/news/posts/${id}`, Post)
		})
};
