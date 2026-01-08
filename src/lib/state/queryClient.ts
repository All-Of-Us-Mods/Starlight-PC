import { QueryClient, QueryCache, MutationCache } from '@tanstack/svelte-query';
import { createAsyncStoragePersister } from '@tanstack/query-async-storage-persister';
import { LazyStore } from '@tauri-apps/plugin-store';
import { showError } from '$lib/utils/toast';

// This will create/load a file named 'query-cache.json' in your app's data directory
const store = new LazyStore('query-cache.json');

export const queryClient = new QueryClient({
	queryCache: new QueryCache({
		onError: (error, query) => {
			// Only show error toast for queries that have already been successful before
			// This prevents showing errors for initial loads that might fail due to network issues
			if (query.state.data !== undefined) {
				showError(error, 'Data fetch failed');
			}
		}
	}),
	mutationCache: new MutationCache({
		onError: (error) => {
			showError(error, 'Operation failed');
		}
	}),
	defaultOptions: {
		queries: {
			gcTime: Infinity,
			staleTime: 1000 * 60 * 5 // 5 minutes
		}
	}
});

// Create the Async Persister for Tauri
export const tauriPersister = createAsyncStoragePersister({
	storage: {
		getItem: async (key) => {
			return await store.get<{ body: string }>(key).then((v) => v?.body ?? null);
		},
		setItem: async (key, value) => {
			await store.set(key, { body: value });
			// Tauri Store saves automatically by default,
			// but you can call .save() to be certain.
			await store.save();
		},
		removeItem: async (key) => {
			await store.delete(key);
			await store.save();
		}
	},
	// Throttling writes to disk for performance
	throttleTime: 1000
});
