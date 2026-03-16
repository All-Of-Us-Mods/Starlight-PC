import { watch, type UnwatchFn } from "@tauri-apps/plugin-fs";
import { info, error as logError } from "@tauri-apps/plugin-log";

type WatchCallback = () => void | Promise<void>;

class FileWatcherManager {
  #watchers = new Map<string, { unwatch: UnwatchFn; callbacks: Set<WatchCallback> }>();

  async watchPath(path: string, callback: WatchCallback, recursive = true): Promise<UnwatchFn> {
    await info(`Setting up file watcher for: ${path}`);

    const existing = this.#watchers.get(path);
    if (existing) {
      existing.callbacks.add(callback);
      await info(
        `Reusing existing watcher for: ${path} (callback count: ${existing.callbacks.size})`,
      );
      return () => this.unwatchPath(path, callback);
    }

    try {
      const unwatch = await watch(
        path,
        () => {
          void info(`File change detected in: ${path}`);
          const entry = this.#watchers.get(path);
          entry?.callbacks.forEach((cb) => {
            void Promise.resolve(cb()).catch((error) => {
              void logError(`Watcher callback failed for ${path}: ${error}`);
            });
          });
        },
        { recursive },
      );

      const callbacks = new Set([callback]);
      this.#watchers.set(path, { unwatch, callbacks });
      await info(`File watcher started for: ${path}`);

      return () => this.unwatchPath(path, callback);
    } catch (err) {
      await logError(`Failed to setup file watcher for ${path}: ${err}`);
      throw err;
    }
  }

  private unwatchPath(path: string, callback: WatchCallback): void {
    const entry = this.#watchers.get(path);
    if (!entry) return;

    entry.callbacks.delete(callback);

    if (entry.callbacks.size === 0) {
      entry.unwatch();
      this.#watchers.delete(path);
      void info(`Stopped file watcher for: ${path}`);
    }
  }
}

export const fileWatcherManager = new FileWatcherManager();

export async function watchDirectory(
  path: string,
  callback: WatchCallback,
  options = { recursive: true },
): Promise<() => void> {
  return fileWatcherManager.watchPath(path, callback, options.recursive);
}
