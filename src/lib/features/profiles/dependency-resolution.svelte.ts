import { modInstallService, type DependencyWithMeta } from './mod-install-service';
import type { ModDependency } from '$lib/features/mods/schema';
import { SvelteSet } from 'svelte/reactivity';

interface DependencyResolutionState {
	isLoadingDeps: boolean;
	resolvedDependencies: DependencyWithMeta[];
	selectedDependencies: SvelteSet<string>;
	installableDependencies: DependencyWithMeta[];
}

export function useDependencyResolution(
	versionInfo: { dependencies: ModDependency[] } | undefined
) {
	const state = $state<DependencyResolutionState>({
		isLoadingDeps: false,
		resolvedDependencies: [],
		selectedDependencies: new SvelteSet(),
		installableDependencies: []
	});

	async function loadDependencies(dependencies: ModDependency[]) {
		state.isLoadingDeps = true;
		try {
			const resolved = await modInstallService.resolveDependencies(dependencies);
			state.resolvedDependencies = resolved;
			state.selectedDependencies = new SvelteSet(
				resolved
					.filter((d: DependencyWithMeta) => d.type !== 'conflict')
					.map((d: DependencyWithMeta) => d.mod_id)
			);
		} catch {
			state.resolvedDependencies = [];
			state.selectedDependencies = new SvelteSet();
		} finally {
			state.isLoadingDeps = false;
		}
	}

	function toggleDependency(modId: string) {
		state.selectedDependencies = new SvelteSet(
			state.selectedDependencies.has(modId)
				? [...state.selectedDependencies].filter((id) => id !== modId)
				: [...state.selectedDependencies, modId]
		);
	}

	$effect(() => {
		if (versionInfo && versionInfo.dependencies.length > 0) {
			loadDependencies(versionInfo.dependencies);
		} else if (versionInfo) {
			state.resolvedDependencies = [];
			state.selectedDependencies = new SvelteSet();
		}
		state.installableDependencies = state.resolvedDependencies.filter(
			(d: DependencyWithMeta) => d.type !== 'conflict'
		);
	});

	return {
		get isLoadingDeps() {
			return state.isLoadingDeps;
		},
		get resolvedDependencies() {
			return state.resolvedDependencies;
		},
		get selectedDependencies() {
			return state.selectedDependencies;
		},
		get installableDependencies() {
			return state.installableDependencies;
		},
		toggleDependency
	};
}
