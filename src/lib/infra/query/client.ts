import { QueryClient } from '@tanstack/svelte-query';
import { persistQueryClient } from '@tanstack/query-persist-client-core';
import { tauriStorePersister } from './persister';

export const queryClient = new QueryClient({
	defaultOptions: {
		queries: {
			refetchOnWindowFocus: false,
			staleTime: 1000 * 60 * 5, // 5 minutes
			gcTime: 1000 * 60 * 60 * 24 * 7, // 1 week
			networkMode: 'always'
		},
		mutations: {
			networkMode: 'always'
		}
	}
});

export function initQueryPersistence() {
	return persistQueryClient({
		queryClient: queryClient as any,
		persister: tauriStorePersister,
		maxAge: 1000 * 60 * 60 * 24 * 7, // 1 week
		dehydrateOptions: {
			shouldDehydrateQuery: (query) => {
				const key = query.queryKey[0];
				return (
					typeof key === 'string' && (key === 'mods' || key === 'news' || key === 'resolved-deps')
				);
			}
		}
	});
}
