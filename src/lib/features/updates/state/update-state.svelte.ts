import { updateService } from '../services/update-service';
import type { UpdateInfo, UpdateStatus, DownloadProgress } from '../update-types';
import { error as logError } from '@tauri-apps/plugin-log';

// ============================================================================
// Update State (Svelte 5 Runes)
// ============================================================================

function createUpdateState() {
	let status = $state<UpdateStatus>('idle');
	let updateInfo = $state<UpdateInfo | null>(null);
	let progress = $state<DownloadProgress>({ downloaded: 0, total: undefined, percent: 0 });
	let dismissed = $state(false);
	let errorMessage = $state<string | null>(null);

	const isVisible = $derived(
		!dismissed &&
			updateInfo !== null &&
			(status === 'idle' || status === 'downloading' || status === 'installing')
	);

	return {
		// Getters
		get status() {
			return status;
		},
		get updateInfo() {
			return updateInfo;
		},
		get progress() {
			return progress;
		},
		get dismissed() {
			return dismissed;
		},
		get errorMessage() {
			return errorMessage;
		},
		get isVisible() {
			return isVisible;
		},

		// Actions
		async check(): Promise<void> {
			if (status === 'checking' || status === 'downloading' || status === 'installing') {
				return;
			}

			status = 'checking';
			errorMessage = null;

			try {
				const info = await updateService.checkForUpdate();
				updateInfo = info;
				status = 'idle';
			} catch (err) {
				status = 'error';
				errorMessage = err instanceof Error ? err.message : 'Failed to check for updates';
				await logError(`Update check failed: ${errorMessage}`);
			}
		},

		async install(): Promise<void> {
			if (!updateInfo || status === 'downloading' || status === 'installing') {
				return;
			}

			status = 'downloading';
			errorMessage = null;
			progress = { downloaded: 0, total: undefined, percent: 0 };

			try {
				await updateService.downloadAndInstall((p) => {
					progress = p;
				});
				// If we get here, relaunch didn't happen (shouldn't normally)
				status = 'installing';
			} catch (err) {
				status = 'error';
				errorMessage = err instanceof Error ? err.message : 'Failed to install update';
				await logError(`Update install failed: ${errorMessage}`);
			}
		},

		dismiss(): void {
			dismissed = true;
		},

		reset(): void {
			status = 'idle';
			updateInfo = null;
			progress = { downloaded: 0, total: undefined, percent: 0 };
			dismissed = false;
			errorMessage = null;
			updateService.clearPendingUpdate();
		}
	};
}

export const updateState = createUpdateState();
