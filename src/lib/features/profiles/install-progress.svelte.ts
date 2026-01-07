import { SvelteMap } from 'svelte/reactivity';
import type { BepInExProgress } from './bepinex-download';

export type InstallState =
	| { status: 'installing'; progress: BepInExProgress }
	| { status: 'error'; message: string };

function createInstallProgressState() {
	const activeInstalls = new SvelteMap<string, InstallState>();

	return {
		get activeInstalls() {
			return activeInstalls;
		},

		setProgress(profileId: string, progress: BepInExProgress) {
			activeInstalls.set(profileId, { status: 'installing', progress });
		},

		setError(profileId: string, message: string) {
			activeInstalls.set(profileId, { status: 'error', message });
		},

		clearProgress(profileId: string) {
			activeInstalls.delete(profileId);
		},

		getState(profileId: string): InstallState | undefined {
			return activeInstalls.get(profileId);
		},

		isInstalling(profileId: string): boolean {
			const state = activeInstalls.get(profileId);
			return state?.status === 'installing';
		},

		hasError(profileId: string): boolean {
			const state = activeInstalls.get(profileId);
			return state?.status === 'error';
		}
	};
}

export const installProgress = createInstallProgressState();
