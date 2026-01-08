import { SvelteMap } from 'svelte/reactivity';
import type { ModDownloadProgress } from './mod-install-service';

export type ModDownloadState =
	| { status: 'downloading'; progress: ModDownloadProgress }
	| { status: 'complete' }
	| { status: 'error'; message: string };

function createModDownloadProgressState() {
	const downloads = new SvelteMap<string, ModDownloadState>();

	return {
		get downloads() {
			return downloads;
		},

		setProgress(modId: string, progress: ModDownloadProgress) {
			if (progress.stage === 'complete') {
				downloads.set(modId, { status: 'complete' });
			} else {
				downloads.set(modId, { status: 'downloading', progress });
			}
		},

		setError(modId: string, message: string) {
			downloads.set(modId, { status: 'error', message });
		},

		clear(modId: string) {
			downloads.delete(modId);
		},

		clearAll() {
			downloads.clear();
		},

		getState(modId: string): ModDownloadState | undefined {
			return downloads.get(modId);
		},

		isDownloading(modId: string): boolean {
			const state = downloads.get(modId);
			return state?.status === 'downloading';
		},

		/** Get stage display text */
		getStageText(stage: ModDownloadProgress['stage']): string {
			switch (stage) {
				case 'connecting':
					return 'Connecting...';
				case 'downloading':
					return 'Downloading...';
				case 'verifying':
					return 'Verifying checksum...';
				case 'writing':
					return 'Writing file...';
				case 'complete':
					return 'Complete';
			}
		}
	};
}

export const modDownloadProgress = createModDownloadProgressState();
