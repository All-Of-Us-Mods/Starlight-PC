import { createMutation, useQueryClient } from '@tanstack/svelte-query';
import { settingsService } from './settings-service';
import type { AppSettings } from './schema';

export function useUpdateSettings() {
	const queryClient = useQueryClient();
	return createMutation<void, Error, Partial<AppSettings>>(() => ({
		mutationFn: settingsService.updateSettings,
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: ['settings'] });
		}
	}));
}
