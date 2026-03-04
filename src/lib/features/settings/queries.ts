import { queryOptions } from '@tanstack/svelte-query';
import { invoke } from '@tauri-apps/api/core';
import { settingsQueryKey } from './settings-keys';
import type { AppSettings } from './schema';

export const settingsQueries = {
	get: () =>
		queryOptions({
			queryKey: settingsQueryKey,
			queryFn: () => invoke<AppSettings>('core_get_settings')
		})
};
