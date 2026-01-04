import { PUBLIC_API_URL } from '$env/static/public';
import { debug, error as logError } from '@tauri-apps/plugin-log';

export async function apiFetch<T>(
	path: string,
	validator: { assert: (data: unknown) => T }
): Promise<T> {
	const url = `${PUBLIC_API_URL}${path}`;
	debug(`Fetching: ${url}`);

	try {
		const response = await fetch(url);

		if (!response.ok) {
			const errorMsg = `HTTP error: ${response.status} ${response.statusText}`;
			logError(`${errorMsg} for ${url}`);
			throw new Error(errorMsg);
		}

		const jsonData = await response.json();
		debug(`Response received for: ${path}`);
		return validator.assert(jsonData);
	} catch (error) {
		if (error instanceof Error) {
			logError(`Request failed for ${path}: ${error.message}`);
		}
		throw error;
	}
}
