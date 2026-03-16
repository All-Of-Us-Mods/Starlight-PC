import type { Profile } from '$lib/features/profiles/schema';

export type InstallTargetSource = 'manual' | 'install-click' | 'profile-context' | 'launch';

interface InstallTargetState {
	profileId: string | null;
	source: InstallTargetSource | null;
	updatedAt: number;
}

const sourcePriority: Record<InstallTargetSource, number> = {
	manual: 5,
	'install-click': 4,
	'profile-context': 3,
	launch: 2
};

let installTarget = $state<InstallTargetState>({
	profileId: null,
	source: null,
	updatedAt: 0
});

function isKnownProfile(profiles: Profile[], profileId: string | null): profileId is string {
	if (!profileId) return false;
	return profiles.some((profile) => profile.id === profileId);
}

export function rememberInstallTarget(profileId: string, source: InstallTargetSource): void {
	if (!profileId) return;

	const current = installTarget;
	if (!current.profileId || !current.source) {
		installTarget = { profileId, source, updatedAt: Date.now() };
		return;
	}

	const currentPriority = sourcePriority[current.source];
	const incomingPriority = sourcePriority[source];

	// Manual selection is sticky until manually changed.
	if (current.source === 'manual' && source !== 'manual') {
		return;
	}

	if (incomingPriority < currentPriority) {
		return;
	}

	// Guard for future sources that might share a priority level:
	// if two different source types have the same priority, the first one wins.
	if (incomingPriority === currentPriority && source !== current.source) {
		return;
	}

	installTarget = { profileId, source, updatedAt: Date.now() };
}

export function getInstallTarget(profiles: Profile[]): string | null {
	return isKnownProfile(profiles, installTarget.profileId) ? installTarget.profileId : null;
}

export function clearInstallTarget(): void {
	installTarget = { profileId: null, source: null, updatedAt: 0 };
}

export function clearIfMissing(profiles: Profile[]): void {
	if (!installTarget.profileId) return;
	if (!isKnownProfile(profiles, installTarget.profileId)) {
		clearInstallTarget();
	}
}
