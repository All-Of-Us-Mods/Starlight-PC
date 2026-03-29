<script lang="ts">
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { createMutation, createQuery, useQueryClient } from '@tanstack/svelte-query';
	import { Debounced, watch } from 'runed';
	import { SvelteMap, SvelteSet } from 'svelte/reactivity';

	import { profileActions } from '$lib/features/profiles/actions';
	import { profileQueries } from '$lib/features/profiles/queries';
	import {
		fetchProfileModUpdates,
		filterProfileMods,
		getProfileModsPagination,
		paginateProfileMods
	} from '$lib/features/profiles/services/profile-detail-view.service';
	import type { Profile, ProfileModUpdatesMap, UnifiedMod } from '$lib/features/profiles/schema';
	import type { Mod } from '$lib/features/mods/schema';
	import { modQueries } from '$lib/features/mods/queries';
	import { mapModsById } from '$lib/features/mods/components/mod-utils';
	import { profileUnifiedModsKey, profilesQueryKey } from '$lib/features/profiles/profile-keys';
	import { rememberInstallTarget } from '$lib/features/mods/state/install-target.svelte';
	import { showError, showSuccess } from '$lib/utils/toast';

	import ProfileModsToolbar from '$lib/features/profiles/components/detail/ProfileModsToolbar.svelte';
	import ProfileModsList from '$lib/features/profiles/components/detail/ProfileModsList.svelte';
	import {
		AlertDialog,
		AlertDialogAction,
		AlertDialogCancel,
		AlertDialogContent,
		AlertDialogDescription,
		AlertDialogFooter,
		AlertDialogHeader,
		AlertDialogTitle
	} from '$lib/components/ui/alert-dialog';

	interface Props {
		profile: Profile;
		isDisabled: boolean;
	}

	const PROFILE_MODS_PAGE_SIZE = 6;
	const queryClient = useQueryClient();
	const lastCleanupSignatureByProfile = new SvelteMap<string, string>();

	let { profile, isDisabled }: Props = $props();

	const unifiedModsQuery = createQuery(() => ({
		...profileQueries.unifiedMods(profile.id, queryClient),
		enabled: !!profile.id
	}));

	const modIds = $derived(Array.from(new Set(profile.mods.map((mod) => mod.mod_id) ?? [])));
	const profileModsQuery = createQuery(() => ({
		queryKey: ['mods', 'profile-batch', profile.id, ...modIds],
		enabled: modIds.length > 0,
		queryFn: async () => {
			const results = await Promise.allSettled(
				modIds.map((id) => queryClient.fetchQuery(modQueries.byId(id)))
			);
			return results
				.filter((result): result is PromiseFulfilledResult<Mod> => result.status === 'fulfilled')
				.map((result) => result.value);
		}
	}));
	const modsMap = $derived(mapModsById(profileModsQuery.data ?? []));

	let searchInput = $state('');
	const debouncedSearch = new Debounced(() => searchInput, 150);
	let currentPage = $state(0);

	let modToDelete = $state<UnifiedMod | null>(null);
	let deleteModDialogOpen = $state(false);
	let isUpdatingAll = $state(false);
	const updatingModIds = new SvelteSet<string>();

	watch(
		() => debouncedSearch.current,
		() => {
			currentPage = 0;
		},
		{ lazy: true }
	);

	const filteredMods = $derived.by(() => {
		const unified = unifiedModsQuery.data ?? [];
		return filterProfileMods(unified, modsMap, debouncedSearch.current);
	});
	const cleanupSignature = $derived.by(() => {
		const profileModsSignature = profile.mods
			.map((mod) => `${mod.mod_id}:${mod.version}:${mod.file ?? ''}`)
			.toSorted()
			.join('|');
		const unifiedModsSignature = (unifiedModsQuery.data ?? [])
			.map((mod) =>
				mod.source === 'managed'
					? `managed:${mod.mod_id}:${mod.version}:${mod.file}`
					: `custom:${mod.file}`
			)
			.toSorted()
			.join('|');
		return `${profile.id}::${profileModsSignature}::${unifiedModsSignature}`;
	});
	const managedModsForUpdates = $derived.by(() => {
		const unified = unifiedModsQuery.data ?? [];
		return unified
			.filter((mod) => mod.source === 'managed')
			.map((mod) => ({
				modId: mod.mod_id,
				installedVersion: mod.version
			}));
	});
	const modUpdatesSignature = $derived(
		managedModsForUpdates
			.map((mod) => `${mod.modId}@${mod.installedVersion}`)
			.toSorted()
			.join('|')
	);
	const modUpdatesQuery = createQuery(() => ({
		queryKey: ['profile-mod-updates', profile.id, modUpdatesSignature],
		enabled: !!profile.id && managedModsForUpdates.length > 0,
		queryFn: () => fetchProfileModUpdates(queryClient, managedModsForUpdates)
	}));
	const modUpdatesQueryKey = $derived(['profile-mod-updates', profile.id, modUpdatesSignature]);
	const modUpdateStatuses = $derived((modUpdatesQuery.data ?? {}) as ProfileModUpdatesMap);
	const displayedMods = $derived(
		paginateProfileMods(filteredMods, currentPage, PROFILE_MODS_PAGE_SIZE)
	);
	const pagination = $derived(
		getProfileModsPagination(filteredMods.length, currentPage, PROFILE_MODS_PAGE_SIZE)
	);

	const isSearching = $derived(debouncedSearch.current.trim().length > 0);
	const updatesAvailableCount = $derived(
		Object.values(modUpdateStatuses).filter((status) => status.isOutdated).length
	);
	const hasManagedModsForUpdates = $derived(managedModsForUpdates.length > 0);
	const isCheckingUpdates = $derived(
		hasManagedModsForUpdates && (modUpdatesQuery.isPending || modUpdatesQuery.isFetching)
	);
	const searchPlaceholder = $derived(
		unifiedModsQuery.data
			? `Search ${unifiedModsQuery.data.length.toLocaleString()} mods...`
			: 'Search mods...'
	);

	const deleteUnifiedMod = createMutation(() => profileActions.deleteUnifiedMod(queryClient));
	const cleanupMissingMods = createMutation(() => profileActions.cleanupMissingMods(queryClient));
	const installMods = createMutation(() => profileActions.installMods(queryClient));

	watch(
		() => cleanupSignature,
		(currentSignature) => {
			if (!currentSignature) return;
			if (lastCleanupSignatureByProfile.get(profile.id) === currentSignature) return;
			lastCleanupSignatureByProfile.set(profile.id, currentSignature);
			void cleanupMissingMods.mutateAsync(profile.id).catch(() => {
				lastCleanupSignatureByProfile.delete(profile.id);
			});
		}
	);

	function goToInstallMods() {
		rememberInstallTarget(profile.id, 'install-click');
		goto(resolve('/explore'));
	}

	function confirmDeleteMod(mod: UnifiedMod) {
		modToDelete = mod;
		deleteModDialogOpen = true;
	}

	function cancelDeleteMod() {
		deleteModDialogOpen = false;
		modToDelete = null;
	}

	async function handleDeleteMod() {
		if (!modToDelete) return;
		deleteModDialogOpen = false;
		try {
			await deleteUnifiedMod.mutateAsync({ profileId: profile.id, mod: modToDelete });
			showSuccess('Mod removed');
		} catch (error) {
			showError(error, 'Remove mod');
		} finally {
			modToDelete = null;
		}
	}

	function applyInstantUpdate(updatedMods: Array<{ modId: string; version: string }>) {
		const nextByModId = new Map(updatedMods.map((mod) => [mod.modId, mod.version]));

		queryClient.setQueryData(profilesQueryKey, (current: Profile[] | undefined) => {
			if (!current) return current;
			return current.map((entry) => {
				if (entry.id !== profile.id) return entry;
				return {
					...entry,
					mods: entry.mods.map((mod) => {
						const nextVersion = nextByModId.get(mod.mod_id);
						return nextVersion ? { ...mod, version: nextVersion } : mod;
					})
				};
			});
		});

		queryClient.setQueryData(profileUnifiedModsKey(profile.id), (current: UnifiedMod[] | undefined) => {
			if (!current) return current;
			return current.map((mod) => {
				if (mod.source !== 'managed') return mod;
				const nextVersion = nextByModId.get(mod.mod_id);
				return nextVersion ? { ...mod, version: nextVersion } : mod;
			});
		});

		queryClient.setQueryData(modUpdatesQueryKey, (current: ProfileModUpdatesMap | undefined) => {
			if (!current) return current;
			const next = { ...current };
			for (const mod of updatedMods) {
				const status = next[mod.modId];
				if (!status) continue;
				next[mod.modId] = {
					...status,
					installedVersion: mod.version,
					latestVersion: mod.version,
					isOutdated: false,
					status: 'ready'
				};
			}
			return next;
		});
	}

	async function handleRefreshUpdates() {
		if (!managedModsForUpdates.length) return;
		await modUpdatesQuery.refetch();
	}

	async function handleUpdateOne(modId: string) {
		const status = modUpdateStatuses[modId];
		if (!status?.isOutdated || !status.latestVersion) return;

		updatingModIds.add(modId);
		try {
			await installMods.mutateAsync({
				profileId: profile.id,
				mods: [{ modId, version: status.latestVersion }]
			});
			applyInstantUpdate([{ modId, version: status.latestVersion }]);
			showSuccess(`Updated ${modsMap.get(modId)?.name ?? modId}`);
			void modUpdatesQuery.refetch();
		} catch (error) {
			showError(error, 'Update mod');
		} finally {
			updatingModIds.delete(modId);
		}
	}

	async function handleUpdateAll() {
		if (updatesAvailableCount === 0) return;

		const modsToUpdate = managedModsForUpdates
			.map((mod) => {
				const status = modUpdateStatuses[mod.modId];
				return status?.isOutdated && status.latestVersion
					? { modId: mod.modId, version: status.latestVersion }
					: null;
			})
			.filter((mod): mod is { modId: string; version: string } => mod !== null);
		if (modsToUpdate.length === 0) return;

		isUpdatingAll = true;
		updatingModIds.clear();
		for (const mod of modsToUpdate) {
			updatingModIds.add(mod.modId);
		}
		try {
			await installMods.mutateAsync({
				profileId: profile.id,
				mods: modsToUpdate
			});
			applyInstantUpdate(modsToUpdate);
			showSuccess(`Updated ${modsToUpdate.length} mod${modsToUpdate.length === 1 ? '' : 's'}`);
			void modUpdatesQuery.refetch();
		} catch (error) {
			showError(error, 'Update all mods');
		} finally {
			isUpdatingAll = false;
			updatingModIds.clear();
		}
	}
</script>

<div class="rounded-lg bg-white/3 p-4">
	<ProfileModsToolbar
		bind:searchInput
		{searchPlaceholder}
		{updatesAvailableCount}
		{isCheckingUpdates}
		{isUpdatingAll}
		onInstallMods={goToInstallMods}
		onRefreshUpdates={handleRefreshUpdates}
		onUpdateAll={handleUpdateAll}
	/>
	<ProfileModsList
		isPending={unifiedModsQuery.isPending}
		{displayedMods}
		{isSearching}
		{profile}
		{modsMap}
		{isDisabled}
		{modUpdateStatuses}
		{updatingModIds}
		{isUpdatingAll}
		showPagination={pagination.showPagination}
		{currentPage}
		totalPages={pagination.totalPages}
		hasNextPage={pagination.hasNextPage}
		onClearSearch={() => (searchInput = '')}
		onInstallMods={goToInstallMods}
		onDeleteMod={confirmDeleteMod}
		onUpdateMod={handleUpdateOne}
		onPrevPage={() => currentPage--}
		onNextPage={() => currentPage++}
	/>
</div>

<AlertDialog bind:open={deleteModDialogOpen}>
	<AlertDialogContent>
		<AlertDialogHeader>
			<AlertDialogTitle>Remove Mod?</AlertDialogTitle>
			<AlertDialogDescription>
				Are you sure you want to remove this mod from the profile? The mod file will be deleted.
			</AlertDialogDescription>
		</AlertDialogHeader>
		<AlertDialogFooter>
			<AlertDialogCancel onclick={cancelDeleteMod}>Cancel</AlertDialogCancel>
			<AlertDialogAction
				onclick={handleDeleteMod}
				class="bg-destructive text-destructive-foreground hover:bg-destructive/90"
			>
				Remove Mod
			</AlertDialogAction>
		</AlertDialogFooter>
	</AlertDialogContent>
</AlertDialog>
