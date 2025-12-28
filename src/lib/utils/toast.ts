import { toast } from 'svelte-sonner';
import { handleError, isNetworkError, isValidationError, isAuthError } from './error-handler';

export function showToastError(error: unknown): void {
	const message = handleError(error);
	const description = getErrorDescription(error);

	toast.error(message, description ? { description } : undefined);
}

export function showToastSuccess(message: string, description?: string): void {
	toast.success(message, description ? { description } : undefined);
}

export function showToastInfo(message: string, description?: string): void {
	toast.info(message, description ? { description } : undefined);
}

export function showToastWarning(message: string, description?: string): void {
	toast.warning(message, description ? { description } : undefined);
}

function getErrorDescription(error: unknown): string | undefined {
	if (isNetworkError(error)) {
		return 'Please check your internet connection and try again.';
	}
	if (isValidationError(error)) {
		return 'Please check your input and try again.';
	}
	if (isAuthError(error)) {
		return 'You may need to log in again.';
	}
	return undefined;
}
