import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { profileService } from './profile-service';

interface GameStatePayload {
	running: boolean;
	profileId?: string;
}

class GameStateService {
	#running = $state(false);
	#runningProfileId = $state<string | null>(null);
	#sessionStartTime = $state<number | null>(null);
	#currentTime = $state(Date.now());
	#unlisten: UnlistenFn | null = null;
	#interval: ReturnType<typeof setInterval> | null = null;

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

	// Internal helper to handle the saving logic
	private async finalizeSession() {
		if (this.#running && this.#runningProfileId) {
			const duration = this.getSessionDuration();
			if (duration > 0) {
				// We use a local variable to avoid race conditions
				// if #runningProfileId changes during the await
				const pid = this.#runningProfileId;
				await profileService.addPlayTime(pid, duration);
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
			// If it was running and is now stopping
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
}

export const gameState = new GameStateService();
