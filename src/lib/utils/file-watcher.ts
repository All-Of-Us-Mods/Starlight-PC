import { watch } from '@tauri-apps/plugin-fs';
import type { UnwatchFn } from '@tauri-apps/plugin-fs';
import { info, error as logError } from '@tauri-apps/plugin-log';

type WatchCallback = () => void;

class FileWatcherManager {
	#watchers = new Map<string, { unwatch: UnwatchFn; callback: WatchCallback; count: number }>();

	async watchPath(path: string, callback: WatchCallback, recursive = true): Promise<UnwatchFn> {
		info(`Setting up file watcher for: ${path}`);

		const existing = this.#watchers.get(path);
		if (existing) {
			existing.count++;
			info(`Reusing existing watcher for: ${path} (count: ${existing.count})`);
			return () => this.unwatchPath(path);
		}

		try {
			const unwatch = await watch(
				path,
				() => {
					info(`File change detected in: ${path}`);
					callback();
				},
				{ recursive }
			);

			this.#watchers.set(path, { unwatch, callback, count: 1 });
			info(`File watcher started for: ${path}`);

			return () => this.unwatchPath(path);
		} catch (err) {
			logError(`Failed to setup file watcher for ${path}: ${err}`);
			throw err;
		}
	}

	private unwatchPath(path: string): void {
		const entry = this.#watchers.get(path);
		if (!entry) return;

		entry.count--;

		if (entry.count <= 0) {
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
