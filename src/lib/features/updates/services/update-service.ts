import type { DownloadProgress, UpdateInfo } from '../update-types';
import { check, type DownloadEvent, type Update } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import { info, error as logError } from '@tauri-apps/plugin-log';

// ============================================================================
// Update Service
// ============================================================================

class UpdateService {
	private pendingUpdate: Update | null = null;

	/**
	 * Check for available updates.
	 * Returns update info if available, null otherwise.
	 */
	async checkForUpdate(): Promise<UpdateInfo | null> {
		try {
			await info('Checking for updates...');
			const update = await check();

			if (update) {
				await info(`Update available: ${update.version}`);
				this.pendingUpdate = update;

				return {
					version: update.version,
					currentVersion: update.currentVersion,
					body: update.body ?? undefined,
					date: update.date ?? undefined
				};
			}

			await info('No updates available');
			return null;
		} catch (error) {
			const message = error instanceof Error ? error.message : String(error);
			await logError(`Failed to check for updates: ${message}`);
		}
		return null;
	}

	/**
	 * Download and install the pending update.
	 * Calls onProgress during download with progress info.
	 * After install, relaunches the app (on non-Windows, Windows auto-exits).
	 */
	async downloadAndInstall(onProgress?: (progress: DownloadProgress) => void): Promise<void> {
		if (!this.pendingUpdate) {
			throw new Error('No pending update to install');
		}

		try {
			await info('Starting update download...');
			let downloaded = 0;
			let contentLength: number | undefined;

			await this.pendingUpdate.downloadAndInstall(async (event: DownloadEvent) => {
				switch (event.event) {
					case 'Started':
						contentLength = event.data.contentLength ?? undefined;
						await info(`Download started, total size: ${contentLength ?? 'unknown'}`);
						break;
					case 'Progress': {
						downloaded += event.data.chunkLength;
						const percent = contentLength ? Math.round((downloaded / contentLength) * 100) : 0;
						onProgress?.({
							downloaded,
							total: contentLength,
							percent
						});
						break;
					}
					case 'Finished':
						await info('Download finished, installing...');
						break;
				}
			});

			await info('Update installed, relaunching...');
			this.pendingUpdate = null;

			// Relaunch the app (on Windows, the installer handles this automatically)
			await relaunch();
		} catch (error) {
			const message = error instanceof Error ? error.message : String(error);
			await logError(`Failed to install update: ${message}`);
			throw error;
		}
	}

	/**
	 * Clear the pending update reference.
	 */
	clearPendingUpdate(): void {
		this.pendingUpdate = null;
	}
}

export const updateService = new UpdateService();
