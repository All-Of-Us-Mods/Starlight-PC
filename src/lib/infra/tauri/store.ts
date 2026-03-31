import { Store } from "@tauri-apps/plugin-store";

const STORE_NAME = "registry.json";

let storeInstance: Store | null = null;
let storePromise: Promise<Store> | null = null;

export async function getStore(): Promise<Store> {
  if (storeInstance) {
    return storeInstance;
  }

  if (storePromise) {
    return storePromise;
  }

  storePromise = Store.load(STORE_NAME).then((store) => {
    storeInstance = store;
    return store;
  });

  return storePromise;
}
