export class AppError extends Error {
	constructor(
		message: string,
		public code: string,
		public originalError?: unknown
	) {
		super(message);
		this.name = 'AppError';
	}
}

export function handleError(error: unknown): string {
	if (error instanceof AppError) {
		return error.message;
	}
	if (error instanceof Error) {
		return error.message;
	}
	if (typeof error === 'string') {
		return error;
	}
	return 'An unexpected error occurred';
}

export function getErrorCode(error: unknown): string {
	if (error instanceof AppError) {
		return error.code;
	}
	return 'UNKNOWN_ERROR';
}

export function isNetworkError(error: unknown): boolean {
	if (error instanceof Error) {
		return (
			error.message.includes('Failed to fetch') ||
			error.message.includes('NetworkError') ||
			error.message.includes('ECONNREFUSED') ||
			error.message.includes('ERR_NETWORK')
		);
	}
	return false;
}

export function isValidationError(error: unknown): boolean {
	if (error instanceof Error) {
		return error.message.includes('validation') || error.message.includes('invalid');
	}
	return false;
}

export function isAuthError(error: unknown): boolean {
	if (error instanceof Error) {
		return error.message.includes('unauthorized') || error.message.includes('forbidden');
	}
	return false;
}
