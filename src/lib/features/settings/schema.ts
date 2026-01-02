import { type } from 'arktype';

export const Settings = type({
	bepinex_url: 'string',
	among_us_path: 'string',
	close_on_launch: 'boolean',
	game_platform: "'steam' | 'epic'",
	cache_bepinex: 'boolean'
});

export type AppSettings = typeof Settings.infer;
export type GamePlatform = 'steam' | 'epic';
