import type { UnifiedMod } from '$lib/features/profiles/schema';

export type ProfileModChip =
	| { id: string; name: string; source: 'managed' }
	| { id: string; name: string; source: 'custom' };

export type UnifiedModsFactory = (diskFiles: string[]) => UnifiedMod[];
