import { QueryClient } from '@tanstack/svelte-query';
import { createAsyncStoragePersister } from '@tanstack/query-async-storage-persister';
import { LazyStore } from '@tauri-apps/plugin-store';

// This will create/load a file named 'query-cache.json' in your app's data directory
const store = new LazyStore('query-cache.json');

export const queryClient = new QueryClient({
	defaultOptions: {
		queries: {
			gcTime: Infinity,
			staleTime: 1000 * 60 * 5, // 5 minutes
			refetchOnWindowFocus: false
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
