import { createAsyncStoragePersister } from "@tanstack/query-async-storage-persister";
import { load } from "@tauri-apps/plugin-store";

let _storePromise: Promise<import("@tauri-apps/plugin-store").Store> | undefined;

function getStore() {
  if (!_storePromise) {
    _storePromise = load("query-cache.json");
  }
  return _storePromise;
}

export const tauriStorePersister = createAsyncStoragePersister({
  storage: {
    getItem: async (key: string) => {
      try {
        const store = await getStore();
        const val = await store.get<{ value: string }>(key);
        return val?.value ?? null;
      } catch {
        return null;
      }
    },
    setItem: async (key: string, value: string) => {
      try {
        const store = await getStore();
        await store.set(key, { value });
        await store.save();
      } catch {}
    },
    removeItem: async (key: string) => {
      try {
        const store = await getStore();
        await store.delete(key);
        await store.save();
      } catch {}
    },
  },
});
