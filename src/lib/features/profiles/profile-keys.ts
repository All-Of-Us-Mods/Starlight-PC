export const profilesQueryKey = ['profiles'] as const;

export const diskFilesQueryKey = ['disk-files'] as const;
export const unifiedModsQueryKey = ['unified-mods'] as const;
export const profileLogsQueryKey = ['profile-logs'] as const;

export const profileDiskFilesKey = (profilePath: string) =>
	[...diskFilesQueryKey, profilePath] as const;
export const profileUnifiedModsKey = (profileId: string) =>
	[...unifiedModsQueryKey, profileId] as const;
export const profileLogKey = (profilePath: string, fileName: string) =>
	[...profileLogsQueryKey, profilePath, fileName] as const;
