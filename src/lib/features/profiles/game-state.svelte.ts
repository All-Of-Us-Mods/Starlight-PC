import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { SvelteMap } from 'svelte/reactivity';
import { profileService } from './profile-service';
import type { BepInExProgress, ModDownloadProgress } from './schema';

// ============ TYPES ============

interface GameStatePayload {
	running: boolean;
	profileId?: string;
}

export type BepInExInstallState =
	| { status: 'installing'; progress: BepInExProgress }
	| { status: 'error'; message: string };

export type ModDownloadState =
	| { status: 'downloading'; progress: ModDownloadProgress }
	| { status: 'complete' }
	| { status: 'error'; message: string };

// ============ CALLBACKS ============

type InvalidateCallback = () => void;
let onProfilesInvalidate: InvalidateCallback | null = null;

/**
 * Register a callback to be called when profiles should be invalidated.
 * This allows the query layer to handle invalidation without services importing queryClient.
 */
export function registerProfilesInvalidateCallback(callback: InvalidateCallback) {
	onProfilesInvalidate = callback;
}

/**
 * Trigger profile invalidation. Called by services when data changes.
 */
export function invalidateProfiles() {
	onProfilesInvalidate?.();
}

// ============ GAME STATE ============

class GameState {
	// Game running state
	#running = $state(false);
	#runningProfileId = $state<string | null>(null);
	#sessionStartTime = $state<number | null>(null);
	#currentTime = $state(Date.now());
	#unlisten: UnlistenFn | null = null;
	#interval: ReturnType<typeof setInterval> | null = null;

	// BepInEx install progress (per profile)
	#bepinexInstalls = new SvelteMap<string, BepInExInstallState>();

	// Mod download progress (per mod)
	#modDownloads = new SvelteMap<string, ModDownloadState>();

	// ============ GAME RUNNING STATE ============

	get running(): boolean {
		return this.#running;
	}

	get runningProfileId(): string | null {
		return this.#runningProfileId;
	}

	isProfileRunning(profileId: string): boolean {
		return this.#running && this.#runningProfileId === profileId;
	}

	getSessionDuration(): number {
		if (!this.#sessionStartTime) return 0;
		return this.#currentTime - this.#sessionStartTime;
	}

	private async finalizeSession() {
		if (this.#running && this.#runningProfileId) {
			const duration = this.getSessionDuration();
			if (duration > 0) {
				const pid = this.#runningProfileId;
				await profileService.addPlayTime(pid, duration);
				onProfilesInvalidate?.();
			}
		}

		this.#sessionStartTime = null;
		if (this.#interval) {
			clearInterval(this.#interval);
			this.#interval = null;
		}
	}

	async init() {
		if (this.#unlisten) return;

		this.#unlisten = await listen<GameStatePayload>('game-state-changed', async (event) => {
			if (this.#running && !event.payload.running) {
				await this.finalizeSession();
			}

			this.#running = event.payload.running;
			this.#runningProfileId = event.payload.profileId ?? null;

			if (this.#running) {
				this.startTimer();
			}
		});
	}

	private startTimer() {
		this.#sessionStartTime = Date.now();
		this.#currentTime = Date.now();
		if (!this.#interval) {
			this.#interval = setInterval(() => {
				this.#currentTime = Date.now();
			}, 1000);
		}
	}

	async setRunningProfile(profileId: string | null): Promise<void> {
		if (!profileId) {
			await this.finalizeSession();
		}

		this.#running = profileId !== null;
		this.#runningProfileId = profileId;

		if (profileId) {
			this.startTimer();
		}
	}

	destroy() {
		if (this.#unlisten) {
			this.#unlisten();
			this.#unlisten = null;
		}
		if (this.#interval) {
			clearInterval(this.#interval);
			this.#interval = null;
		}
	}

	// ============ BEPINEX INSTALL PROGRESS ============

	get bepinexInstalls(): SvelteMap<string, BepInExInstallState> {
		return this.#bepinexInstalls;
	}

	setBepInExProgress(profileId: string, progress: BepInExProgress) {
		this.#bepinexInstalls.set(profileId, { status: 'installing', progress });
	}

	setBepInExError(profileId: string, message: string) {
		this.#bepinexInstalls.set(profileId, { status: 'error', message });
	}

	clearBepInExProgress(profileId: string) {
		this.#bepinexInstalls.delete(profileId);
	}

	getBepInExState(profileId: string): BepInExInstallState | undefined {
		return this.#bepinexInstalls.get(profileId);
	}

	isBepInExInstalling(profileId: string): boolean {
		const state = this.#bepinexInstalls.get(profileId);
		return state?.status === 'installing';
	}

	hasBepInExError(profileId: string): boolean {
		const state = this.#bepinexInstalls.get(profileId);
		return state?.status === 'error';
	}

	// ============ MOD DOWNLOAD PROGRESS ============

	get modDownloads(): SvelteMap<string, ModDownloadState> {
		return this.#modDownloads;
	}

	setModDownloadProgress(modId: string, progress: ModDownloadProgress) {
		if (progress.stage === 'complete') {
			this.#modDownloads.set(modId, { status: 'complete' });
		} else {
			this.#modDownloads.set(modId, { status: 'downloading', progress });
		}
	}

	setModDownloadError(modId: string, message: string) {
		this.#modDownloads.set(modId, { status: 'error', message });
	}

	clearModDownload(modId: string) {
		this.#modDownloads.delete(modId);
	}

	clearAllModDownloads() {
		this.#modDownloads.clear();
	}

	getModDownloadState(modId: string): ModDownloadState | undefined {
		return this.#modDownloads.get(modId);
	}

	isModDownloading(modId: string): boolean {
		const state = this.#modDownloads.get(modId);
		return state?.status === 'downloading';
	}

	getModDownloadStageText(stage: ModDownloadProgress['stage']): string {
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
}

export const gameState = new GameState();
