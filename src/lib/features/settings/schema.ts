import { type } from 'arktype';

export const Settings = type({
	bepinex_url: 'string',
	among_us_path: 'string',
	close_on_launch: 'boolean',
	game_platform: "'steam' | 'epic'",
	cache_bepinex: 'boolean',
	copy_game_files: "'cache' | 'ignore'"
});

export type AppSettings = typeof Settings.infer;
export type GamePlatform = 'steam' | 'epic';
export type CopyGameFiles = 'cache' | 'ignore';
