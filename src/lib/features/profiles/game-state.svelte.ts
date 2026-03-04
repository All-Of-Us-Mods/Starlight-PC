import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { SvelteMap } from 'svelte/reactivity';
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
let runningProfileId = $state<string | null>(null);
let sessionStartTime = $state<number | null>(null);
let currentTime = $state(Date.now());

const bepinexInstalls = new SvelteMap<string, BepInExInstallState>();
const modDownloads = new SvelteMap<string, ModDownloadState>();

let onProfilesInvalidate: InvalidateCallback | null = null;
let unlistenGameState: UnlistenFn | null = null;
let ticker: ReturnType<typeof setInterval> | null = null;

export function registerProfilesInvalidateCallback(callback: InvalidateCallback) {
	onProfilesInvalidate = callback;
}

function notifyProfilesInvalidated() {
	onProfilesInvalidate?.();
}

function getSessionDuration() {
	if (!sessionStartTime) return 0;
	return currentTime - sessionStartTime;
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

function startSessionTimer() {
	if (sessionStartTime) return;
	sessionStartTime = Date.now();
	currentTime = sessionStartTime;
	if (!ticker) {
		ticker = setInterval(() => {
			currentTime = Date.now();
		}, 1000);
	}
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
			const wasRunning = running;
			const prevProfileId = runningProfileId;
			const prevDuration = getSessionDuration();

			if (wasRunning && !event.payload.running) {
				sessionStartTime = null;
				if (ticker) {
					clearInterval(ticker);
					ticker = null;
				}
			}

			running = event.payload.running;
			runningCount = event.payload.running_count ?? (event.payload.running ? 1 : 0);
			profileInstanceCounts = event.payload.profile_instance_counts ?? {};

			const runningProfileIds = Object.entries(profileInstanceCounts)
				.filter(([, count]) => count > 0)
				.map(([profileId]) => profileId);

			if (runningProfileIds.length === 1) {
				runningProfileId = runningProfileIds[0] ?? null;
			} else if (runningProfileIds.length === 0) {
				runningProfileId = null;
			} else if (runningProfileId && !runningProfileIds.includes(runningProfileId)) {
				runningProfileId = null;
			}

			if (running && sessionStartTime === null) {
				startSessionTimer();
			}

			if (wasRunning && !event.payload.running && prevProfileId) {
				await finalizeSession(prevProfileId, prevDuration);
			}
		});
	},
	destroy: () => {
		unlistenGameState?.();
		unlistenGameState = null;
		if (ticker) {
			clearInterval(ticker);
			ticker = null;
		}
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
