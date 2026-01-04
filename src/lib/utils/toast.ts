import { toast } from 'svelte-sonner';
import { error as logError } from '@tauri-apps/plugin-log';

// ============================================================================
// Error Handling
// ============================================================================

function handleError(err: unknown): string {
	if (err instanceof Error) return err.message;
	if (typeof err === 'string') return err;
	return 'An unexpected error occurred';
}

function isNetworkError(err: unknown): boolean {
	if (err instanceof Error) {
		return (
			err.message.includes('Failed to fetch') ||
			err.message.includes('NetworkError') ||
			err.message.includes('ECONNREFUSED') ||
			err.message.includes('ERR_NETWORK')
		);
	}
	return false;
}

function getErrorDescription(err: unknown): string | undefined {
	if (isNetworkError(err)) return 'Please check your internet connection and try again.';
	return undefined;
}

// ============================================================================
// Toast Notifications
// ============================================================================

export function showError(err: unknown, context?: string): void {
	const message = handleError(err);
	const description = getErrorDescription(err);

	const logMessage = context ? `[${context}] ${message}` : message;
	logError(logMessage);

	toast.error(message, description ? { description } : undefined);
}

export function showSuccess(message: string, description?: string): void {
	toast.success(message, description ? { description } : undefined);
}

export function showInfo(message: string, description?: string): void {
	toast.info(message, description ? { description } : undefined);
}

export function showWarning(message: string, description?: string): void {
	toast.warning(message, description ? { description } : undefined);
}
