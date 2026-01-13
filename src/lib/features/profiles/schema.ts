import { type } from 'arktype';

export const ProfileModEntry = type({
	mod_id: 'string',
	version: 'string',
	'file?': 'string' // The installed filename
});

export const ProfileEntry = type({
	id: 'string',
	name: 'string <= 100',
	path: 'string',
	created_at: 'number',
	'last_launched_at?': 'number',
	'bepinex_installed?': 'boolean',
	'total_play_time?': 'number',
	mods: type(ProfileModEntry.array())
});

export type Profile = typeof ProfileEntry.infer;
export type ProfileMod = typeof ProfileModEntry.infer;

export type UnifiedMod =
	| { source: 'managed'; mod_id: string; version: string; file: string }
	| { source: 'custom'; file: string };

// Progress types for BepInEx installation
export interface BepInExProgress {
	stage: 'downloading' | 'extracting' | 'complete';
	progress: number;
	message: string;
}

// Progress types for mod downloads
export interface ModDownloadProgress {
	mod_id: string;
	downloaded: number;
	total: number | null;
	progress: number; // 0-100
	stage: 'connecting' | 'downloading' | 'verifying' | 'writing' | 'complete';
}
