import { showError } from '$lib/utils/toast';
import type { Profile } from '$lib/features/profiles/schema';

interface ShellControllerDeps {
	launchProfile: (profile: Profile) => Promise<void>;
}

export function getSidebarWidth(isMaximized: boolean): string {
	return isMaximized ? '100%' : '400px';
}

export function canLaunchProfile(activeProfile: Profile | null): boolean {
	return !!activeProfile;
}

export function shouldFinalizeSidebarTransition(
	event: TransitionEvent,
	sidebarOpen: boolean
): boolean {
	return event.propertyName === 'width' && !sidebarOpen;
}

export function createShellController(deps: ShellControllerDeps) {
	return {
		async launchActiveProfile(activeProfile: Profile | null): Promise<void> {
			if (!activeProfile) return;

			try {
				await deps.launchProfile(activeProfile);
			} catch (error) {
				showError(error);
			}
		}
	};
}
