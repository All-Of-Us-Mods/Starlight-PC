export type UpdateStatus = 'idle' | 'checking' | 'downloading' | 'installing' | 'error';

export interface UpdateInfo {
	version: string;
	currentVersion: string;
	body: string | undefined;
	date: string | undefined;
}

export interface DownloadProgress {
	downloaded: number;
	total: number | undefined;
	percent: number;
}
