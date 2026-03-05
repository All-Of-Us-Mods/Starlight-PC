import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { SvelteMap, SvelteSet } from 'svelte/reactivity';
import type { BepInExProgress, ModDownloadProgress } from './schema';
import { rustInvoke } from '$lib/infra/rust/invoke';

type InvalidateCallback = () => void;

interface GameStatePayload {
	running: boolean;
	running_count?: number;
	profile_instance_counts?: Record<string, number>;
}

type BepInExInstallState =
	| { status: 'installing'; progress: BepInExProgress }
	| { status: 'error'; message: string };

type ModDownloadState =
	| { status: 'downloading'; progress: ModDownloadProgress }
	| { status: 'complete' }
	| { status: 'error'; message: string };

let running = $state(false);
let runningCount = $state(0);
let profileInstanceCounts = $state<Record<string, number>>({});
let currentTime = $state(Date.now());

const bepinexInstalls = new SvelteMap<string, BepInExInstallState>();
const modDownloads = new SvelteMap<string, ModDownloadState>();
const activeProfileSessions = new SvelteMap<string, number>();

let onProfilesInvalidate: InvalidateCallback | null = null;
let unlistenGameState: UnlistenFn | null = null;
let ticker: ReturnType<typeof setInterval> | null = null;

export function registerProfilesInvalidateCallback(callback: InvalidateCallback) {
	onProfilesInvalidate = callback;
}

function notifyProfilesInvalidated() {
	onProfilesInvalidate?.();
}

function getSessionDuration(profileId?: string) {
	if (!profileId) {
		return Array.from(activeProfileSessions.values()).reduce(
			(maxDuration, startedAt) => Math.max(maxDuration, currentTime - startedAt),
			0
		);
	}
	const startedAt = activeProfileSessions.get(profileId);
	return startedAt ? currentTime - startedAt : 0;
}

async function finalizeSession(profileId: string, durationMs: number) {
	if (durationMs <= 0) return;
	try {
		await rustInvoke('profiles_add_play_time', {
			profileId,
			durationMs
		});
		notifyProfilesInvalidated();
	} catch (error) {
		console.error('[gameState] Failed to persist play time', error);
	}
}

function startTicker() {
	if (!ticker) {
		ticker = setInterval(() => {
			currentTime = Date.now();
		}, 1000);
	}
}

function stopTickerIfIdle() {
	if (activeProfileSessions.size > 0 || !ticker) return;
	clearInterval(ticker);
	ticker = null;
}

export const gameState = {
	get running() {
		return running;
	},
	get runningCount() {
		return runningCount;
	},
	getSessionDuration,
	getProfileRunningInstanceCount(profileId: string) {
		return profileInstanceCounts[profileId] ?? 0;
	},
	isProfileRunning(profileId: string) {
		return (profileInstanceCounts[profileId] ?? 0) > 0;
	},
	init: async () => {
		if (unlistenGameState) return;
		unlistenGameState = await listen<GameStatePayload>('game-state-changed', async (event) => {
			const previousCounts = profileInstanceCounts;
			const nextCounts = event.payload.profile_instance_counts ?? {};
			const touchedProfileIds = new SvelteSet([
				...Object.keys(previousCounts),
				...Object.keys(nextCounts)
			]);

			running = event.payload.running;
			runningCount = event.payload.running_count ?? (event.payload.running ? 1 : 0);
			profileInstanceCounts = nextCounts;
			currentTime = Date.now();

			const finalizedSessions: Array<Promise<void>> = [];
			for (const profileId of touchedProfileIds) {
				const previousCount = previousCounts[profileId] ?? 0;
				const nextCount = nextCounts[profileId] ?? 0;
				const startedAt = activeProfileSessions.get(profileId);

				if (previousCount <= 0 && nextCount > 0) {
					activeProfileSessions.set(profileId, currentTime);
					startTicker();
					continue;
				}

				if (previousCount > 0 && nextCount <= 0 && startedAt) {
					activeProfileSessions.delete(profileId);
					finalizedSessions.push(finalizeSession(profileId, currentTime - startedAt));
				}
			}

			if (finalizedSessions.length > 0) {
				await Promise.all(finalizedSessions);
			}
			stopTickerIfIdle();
		});
	},
	destroy: () => {
		unlistenGameState?.();
		unlistenGameState = null;
		activeProfileSessions.clear();
		stopTickerIfIdle();
	},
	getBepInExState(profileId: string): BepInExInstallState | undefined {
		return bepinexInstalls.get(profileId);
	},
	setBepInExProgress(profileId: string, progress: BepInExProgress) {
		bepinexInstalls.set(profileId, { status: 'installing', progress });
	},
	setBepInExError(profileId: string, message: string) {
		bepinexInstalls.set(profileId, { status: 'error', message });
	},
	clearBepInExProgress(profileId: string) {
		bepinexInstalls.delete(profileId);
	},
	getModDownloadState(modId: string): ModDownloadState | undefined {
		return modDownloads.get(modId);
	},
	setModDownloadProgress(modId: string, progress: ModDownloadProgress) {
		if (progress.stage === 'complete') {
			modDownloads.set(modId, { status: 'complete' });
		} else {
			modDownloads.set(modId, { status: 'downloading', progress });
		}
	},
	setModDownloadError(modId: string, message: string) {
		modDownloads.set(modId, { status: 'error', message });
	},
	clearModDownload(modId: string) {
		modDownloads.delete(modId);
	},
	getModDownloadStageText(stage: ModDownloadProgress['stage']) {
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
			default:
				return '';
		}
	}
};
