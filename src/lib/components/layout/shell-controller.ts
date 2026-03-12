import { showError, showSuccess } from '$lib/utils/toast';
import type { Profile } from '$lib/features/profiles/schema';

interface ShellControllerDeps {
	launchProfile: (profile: Profile) => Promise<void>;
	stopAllInstances: () => Promise<number>;
}

export function getSidebarWidth(isMaximized: boolean): string {
	return isMaximized ? '100%' : '400px';
}

export function canControlGame(activeProfile: Profile | null, hasStoppableRunning: boolean): boolean {
	return hasStoppableRunning || !!activeProfile;
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
		},
		async stopRunningInstances(): Promise<void> {
			try {
				const stoppedCount = await deps.stopAllInstances();
				showSuccess(
					stoppedCount === 1
						? 'Stopped 1 desktop instance'
						: `Stopped ${stoppedCount} desktop instances`
				);
			} catch (error) {
				showError(error);
			}
		}
	};
}
