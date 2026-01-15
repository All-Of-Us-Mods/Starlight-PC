import { watch } from '@tauri-apps/plugin-fs';
import type { UnwatchFn } from '@tauri-apps/plugin-fs';
import { info, error as logError } from '@tauri-apps/plugin-log';

type WatchCallback = () => void | Promise<void>;

class FileWatcherManager {
	#watchers = new Map<string, { unwatch: UnwatchFn; callbacks: Set<WatchCallback> }>();

	async watchPath(path: string, callback: WatchCallback, recursive = true): Promise<UnwatchFn> {
		info(`Setting up file watcher for: ${path}`);

		const existing = this.#watchers.get(path);
		if (existing) {
			existing.callbacks.add(callback);
			info(`Reusing existing watcher for: ${path} (callback count: ${existing.callbacks.size})`);
			return () => this.unwatchPath(path, callback);
		}

		try {
			const unwatch = await watch(
				path,
				() => {
					info(`File change detected in: ${path}`);
					const entry = this.#watchers.get(path);
					entry?.callbacks.forEach((cb) => cb());
				},
				{ recursive }
			);

			const callbacks = new Set([callback]);
			this.#watchers.set(path, { unwatch, callbacks });
			info(`File watcher started for: ${path}`);

			return () => this.unwatchPath(path, callback);
		} catch (err) {
			logError(`Failed to setup file watcher for ${path}: ${err}`);
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
			info(`Stopped file watcher for: ${path}`);
		}
	}
}

export const fileWatcherManager = new FileWatcherManager();

export async function watchDirectory(
	path: string,
	callback: WatchCallback,
	options = { recursive: true }
): Promise<() => void> {
	return fileWatcherManager.watchPath(path, callback, options.recursive);
}
