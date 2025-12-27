import { type } from 'arktype';

export const ExternalLink = type({
	type: 'string',
	url: 'string'
});

export const ModResponseLinks = type({
	self: 'string',
	info: 'string',
	thumbnail: 'string',
	versions: 'string'
});

export const ModResponse = type({
	'status?': 'string', // Maps to db.PublicationStatus
	id: 'string <= 100',
	name: 'string <= 100',
	author: 'string <= 100',
	description: 'string <= 500',
	created_at: 'number',
	updated_at: 'number',
	downloads: 'number',
	_links: ModResponseLinks
});

export const ModInfoResponse = type({
	long_description: 'string <= 20000',
	license: 'string <= 100',
	links: type(ExternalLink.array()),
	tags: 'string[]'
});

export const ModDependency = type({
	mod_id: 'string',
	version_constraint: 'string',
	type: 'string'
});

export const ModVersionInfo = type({
	file_name: 'string',
	changelog: 'string',
	checksum: 'string',
	download_url: 'string',
	dependencies: type(ModDependency.array())
});

export const ModVersion = type({
	name: 'string',
	version: 'string',
	platform: 'string',
	downloads: 'number',
	created_at: 'number',
	_links: {
		self: 'string',
		info: 'string'
	}
});

// TypeScript Types
export type Mod = typeof ModResponse.infer;
export type ModInfo = typeof ModInfoResponse.infer;
export type ExternalLink = typeof ExternalLink.infer;
export type ModDependency = typeof ModDependency.infer;
export type ModVersionInfo = typeof ModVersionInfo.infer;
export type ModVersion = typeof ModVersion.infer;
