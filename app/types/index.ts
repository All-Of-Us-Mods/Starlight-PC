export interface Mod {
	mod_id?: string | number
	mod_name?: string
	author?: string
	thumbnail?: string
	description?: string
	created_at?: number | string
	updated_at?: number | string
	downloads?: number
}

export interface ModVersion {
	self?: string
	mod?: string
	platform?: string
	version?: string
	file_name?: string
	checksum?: string
	downloads?: number
	created_at?: number | string
}

export interface ProfileEntry {
	id: string
	name: string
	path: string
	created_at: number
}
