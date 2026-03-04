import { PUBLIC_API_URL } from '$env/static/public';
import { invoke } from '@tauri-apps/api/core';
import { debug, error as logError } from '@tauri-apps/plugin-log';

export async function apiFetch<T>(
	path: string,
	validator: { assert: (data: unknown) => T }
): Promise<T> {
	const url = `${PUBLIC_API_URL}${path}`;
	debug(`Fetching: ${url}`);

	try {
		const jsonData = await invoke<unknown>('core_api_get', {
			args: { apiBaseUrl: PUBLIC_API_URL, path }
		});
		debug(`Response received for: ${path}`);
		return validator.assert(jsonData);
	} catch (error) {
		if (error instanceof Error) {
			logError(`Request failed for ${path}: ${error.message}`);
		}
		throw error;
	}
}
