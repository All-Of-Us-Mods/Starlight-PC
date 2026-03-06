import type { QueryClient } from '@tanstack/svelte-query';
import { exists } from '@tauri-apps/plugin-fs';
import { join } from '@tauri-apps/api/path';
import { rustInvoke } from '$lib/infra/rust/invoke';
import { profileDiskFilesKey, profilesQueryKey } from '../profile-keys';
import type { Profile } from '../schema';

type ProfileSummary = Pick<Profile, 'id' | 'path'>;

export async function getProfileById(profileId: string): Promise<Profile | null> {
	return rustInvoke('profiles_get_by_id', { id: profileId });
}

export async function assertPathExists(path: string, message: string) {
	if (!(await exists(path))) {
		throw new Error(message);
	}
}

export function getProfilePathFromCache(
	queryClient: QueryClient,
	profileId: string
): string | undefined {
	const profiles = queryClient.getQueryData<ProfileSummary[]>(profilesQueryKey);
	return profiles?.find((profile) => profile.id === profileId)?.path;
}

export async function invalidateProfileAndDiskQueries(
	queryClient: QueryClient,
	args: { profileId: string; profilePath?: string }
) {
	await queryClient.invalidateQueries({ queryKey: profilesQueryKey });
	const profilePath = args.profilePath ?? getProfilePathFromCache(queryClient, args.profileId);
	if (profilePath) {
		await queryClient.invalidateQueries({ queryKey: profileDiskFilesKey(profilePath) });
	}
}

export function buildProfileFilePath(profilePath: string, fileName: string): string {
	const normalized =
		profilePath.endsWith('/') || profilePath.endsWith('\\') ? profilePath : `${profilePath}/`;
	return `${normalized}${fileName}`;
}

export function buildCustomIconFilePath(profilePath: string, extension: string): string {
	return buildProfileFilePath(profilePath, `icon${extension}`);
}

export function resolveGameExecutablePath(gamePath: string): Promise<string> {
	return join(gamePath, 'Among Us.exe');
}

export function resolveBepInExDllPath(profilePath: string): Promise<string> {
	return join(profilePath, 'BepInEx', 'core', 'BepInEx.Unity.IL2CPP.dll');
}

export function resolveDotnetDir(profilePath: string): Promise<string> {
	return join(profilePath, 'dotnet');
}

export function resolveCoreClrPath(dotnetDir: string): Promise<string> {
	return join(dotnetDir, 'coreclr.dll');
}

export function resolveProfilePluginPath(profilePath: string, fileName: string): Promise<string> {
	return join(profilePath, 'BepInEx', 'plugins', fileName);
}

export function resolveProfileLogsDir(profilePath: string): Promise<string> {
	return join(profilePath, 'BepInEx');
}

export function resolveProfileLogPath(profilePath: string, fileName: string): Promise<string> {
	return join(profilePath, 'BepInEx', fileName);
}
