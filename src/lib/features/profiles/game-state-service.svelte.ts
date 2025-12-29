import { listen, type UnlistenFn } from '@tauri-apps/api/event';

class GameStateService {
	#running = $state(false);
	#unlisten: UnlistenFn | null = null;

	get running(): boolean {
		if (!this.#unlisten) {
			console.warn('GameStateService not initialized yet');
		}
		return this.#running;
	}

	async init() {
		if (this.#unlisten) return;

		this.#unlisten = await listen<{ running: boolean }>('game-state-changed', (event) => {
			this.#running = event.payload.running;
		});
	}

	destroy() {
		if (this.#unlisten) {
			this.#unlisten();
			this.#unlisten = null;
		}
	}
}

export const gameState = new GameStateService();
