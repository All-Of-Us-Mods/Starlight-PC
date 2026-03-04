import { fetch as tauriFetch } from '@tauri-apps/plugin-http';

const DEFAULT_API_BASE_URL = 'https://starlight.allofus.dev';

export function apiBaseUrl(): string {
	const raw = import.meta.env.PUBLIC_API_URL;
	if (typeof raw === 'string' && raw.trim().length > 0) {
		return raw.trim();
	}
	return DEFAULT_API_BASE_URL;
}

export class FetchApiError extends Error {
	path: string;
	status?: number;
	cause?: unknown;

	constructor(message: string, path: string, status?: number, cause?: unknown) {
		super(message);
		this.name = 'FetchApiError';
		this.path = path;
		this.status = status;
		this.cause = cause;
	}
}

export function resolveApiUrl(pathOrUrl: string): string {
	if (pathOrUrl.startsWith('http://') || pathOrUrl.startsWith('https://')) {
		return pathOrUrl;
	}
	const base = apiBaseUrl().replace(/\/+$/, '');
	const route = pathOrUrl.startsWith('/') ? pathOrUrl : `/${pathOrUrl}`;
	return `${base}${route}`;
}

export async function apiFetch<T>(
	path: string,
	validator: { assert: (data: unknown) => T }
): Promise<T> {
	const url = resolveApiUrl(path);
	let response: Response;

	try {
		response = await tauriFetch(url);
	} catch (cause) {
		throw new FetchApiError('Network request failed', path, undefined, cause);
	}

	if (!response.ok) {
		throw new FetchApiError(
			`HTTP ${response.status} ${response.statusText}`,
			path,
			response.status
		);
	}

	let payload: unknown;
	try {
		payload = await response.json();
	} catch (cause) {
		throw new FetchApiError('Response was not valid JSON', path, response.status, cause);
	}

	try {
		return validator.assert(payload);
	} catch (cause) {
		throw new FetchApiError('Response validation failed', path, response.status, cause);
	}
}
