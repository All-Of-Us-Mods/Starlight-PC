import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

export type EpicLoginEventHandlers = {
	onStarted?: () => void;
	onSuccess?: () => void;
	onError?: (error: string) => void;
	onCancelled?: () => void;
};

class EpicService {
	isLoggedIn = () => invoke<boolean>('epic_is_logged_in');
	login = (code: string) => invoke<void>('epic_login_with_code', { code });
	loginWithWebview = () => invoke<void>('epic_login_with_webview');
	logout = () => invoke<void>('epic_logout');
	getAuthUrl = () => invoke<string>('get_epic_auth_url');
	tryRestoreSession = () => invoke<boolean>('epic_try_restore_session');

	async ensureLoggedIn(): Promise<void> {
		if (!(await this.tryRestoreSession())) {
			throw new Error('Not logged into Epic Games');
		}
	}

	/** Subscribe to webview login events. Returns cleanup function. */
	subscribeToLoginEvents(handlers: EpicLoginEventHandlers): () => void {
		const promises: Promise<UnlistenFn>[] = [];

		if (handlers.onStarted) {
			promises.push(listen('epic-login-started', handlers.onStarted));
		}
		if (handlers.onSuccess) {
			promises.push(listen('epic-login-success', handlers.onSuccess));
		}
		if (handlers.onError) {
			promises.push(listen<string>('epic-login-error', (e) => handlers.onError?.(e.payload)));
		}
		if (handlers.onCancelled) {
			promises.push(listen('epic-login-cancelled', handlers.onCancelled));
		}

		return () => promises.forEach((p) => p.then((fn) => fn()));
	}
}

export const epicService = new EpicService();
