import { type } from 'arktype';

export const ProfileModEntry = type({
	mod_id: 'string',
	version: 'string'
});

export const ProfileEntry = type({
	id: 'string',
	name: 'string <= 100',
	path: 'string',
	created_at: 'number',
	'last_launched_at?': 'number',
	'bepinex_installed?': 'boolean',
	mods: type(ProfileModEntry.array())
});

export type Profile = typeof ProfileEntry.infer;
export type ProfileMod = typeof ProfileModEntry.infer;
