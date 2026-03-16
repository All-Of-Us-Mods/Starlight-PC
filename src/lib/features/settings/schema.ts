import { type } from 'arktype';

export const Settings = type({
	bepinex_url: 'string',
	among_us_path: 'string',
	close_on_launch: 'boolean',
	allow_multi_instance_launch: 'boolean',
	game_platform: "'steam' | 'epic' | 'xbox'",
	cache_bepinex: 'boolean',
	'xbox_app_id?': 'string'
});

export type AppSettings = typeof Settings.infer;

export type GamePlatform = AppSettings['game_platform'];
