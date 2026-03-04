import { rustQueryOptions } from '$lib/infra/rust/query';
import { settingsQueryKey } from './settings-keys';

export const settingsQueries = {
	get: () =>
		rustQueryOptions({
			queryKey: settingsQueryKey,
			command: 'core_get_settings'
		})
};
