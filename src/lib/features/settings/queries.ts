import { queryOptions } from '@tanstack/svelte-query';
import { settingsService } from './settings-service';

export const settingsQueries = {
	get: () =>
		queryOptions({
			queryKey: ['settings'] as const,
			queryFn: () => settingsService.getSettings()
		})
};
