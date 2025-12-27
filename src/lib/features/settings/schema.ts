import { type } from 'arktype';

export const Settings = type({
	bepinex_url: 'string',
	bepinex_version: 'string',
	among_us_path: 'string',
	close_on_launch: 'boolean'
});

export type AppSettings = typeof Settings.infer;
