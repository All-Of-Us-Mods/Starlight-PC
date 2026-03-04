import type { Profile } from '../schema';

export function findProfileById(
	profiles: Profile[] | undefined,
	profileId: string
): Profile | null {
	if (!profiles) return null;
	return profiles.find((profile) => profile.id === profileId) ?? null;
}

export function getMostRecentlyCreatedProfile(profiles: Profile[] | undefined): Profile | null {
	if (!profiles || profiles.length === 0) return null;
	return [...profiles].toSorted((a, b) => b.created_at - a.created_at)[0] ?? null;
}
