import type { ModDependency } from '$lib/features/mods/schema';
import type { Profile, ProfileIconSelection, UnifiedMod } from '$lib/features/profiles/schema';
import type { AppSettings, GamePlatform } from '$lib/features/settings/schema';

export type ResolvedDependency = {
	mod_id: string;
	modName: string;
	resolvedVersion: string;
	type: 'required' | 'optional' | 'conflict';
};

export type LaunchWorkflowResult = {
	close_on_launch: boolean;
};

export type InstalledProfileMod = {
	mod_id: string;
	version: string;
	file_name: string;
};

export type InstallProfileModsArgs = {
	profileId: string;
	profilePath: string;
	mods: Array<{ modId: string; version: string }>;
};

export type RustCommandMap = {
	core_get_settings: { args: void; result: AppSettings };
	core_update_settings: { args: { updates: Partial<AppSettings> }; result: AppSettings };
	core_get_bepinex_cache_path: { args: void; result: string };
	core_get_app_data_dir: { args: void; result: string };
	core_auto_detect_bepinex_architecture: { args: { gamePath: string }; result: string | null };

	platform_detect_among_us: { args: void; result: string | null };
	platform_detect_game_store: { args: { path: string }; result: GamePlatform };

	profiles_get_dir: { args: void; result: string };
	profiles_list: { args: void; result: Profile[] };
	profiles_get_active: { args: void; result: Profile | null };
	profiles_create: { args: { name: string }; result: Profile };
	profiles_install_bepinex: { args: { profileId: string; profilePath: string }; result: void };
	profiles_delete: { args: { profileId: string }; result: void };
	profiles_rename: { args: { profileId: string; newName: string }; result: void };
	profiles_update_icon: {
		args: { profileId: string; selection: ProfileIconSelection };
		result: void;
	};
	profiles_add_mod: { args: { profileId: string; modId: string; version: string; file: string }; result: void };
	profiles_remove_mod: { args: { profileId: string; modId: string }; result: void };
	profiles_add_play_time: { args: { profileId: string; durationMs: number }; result: void };
	profiles_get_mod_files: { args: { profilePath: string }; result: string[] };
	profiles_get_log: { args: { profilePath: string; fileName: string }; result: string };
	profiles_read_binary_file: { args: { path: string }; result: Uint8Array };
	profiles_get_unified_mods: { args: { profileId: string }; result: UnifiedMod[] };
	profiles_cleanup_missing_mods: { args: { profileId: string }; result: void };
	profiles_delete_unified_mod: { args: { profileId: string; modEntry: UnifiedMod }; result: void };
	profiles_export_zip: { args: { profileId: string; destination: string }; result: void };
	profiles_import_zip: { args: { zipPath: string }; result: Profile };
	profiles_update_last_launched: { args: { profileId: string }; result: void };

	modding_bepinex_cache_download: {
		args: { url: string; cachePath: string };
		result: void;
	};
	modding_bepinex_cache_clear: { args: { cachePath: string }; result: void };
	modding_bepinex_cache_exists: { args: { cachePath: string }; result: boolean };
	modding_resolve_dependencies: { args: { dependencies: ModDependency[] }; result: ResolvedDependency[] };
	modding_install_profile_mods: { args: InstallProfileModsArgs; result: InstalledProfileMod[] };

	game_launch_profile: {
		args: { profileId: string; profilePath: string };
		result: LaunchWorkflowResult;
	};
	game_launch_vanilla_workflow: { args: void; result: LaunchWorkflowResult };

	epic_is_logged_in: { args: void; result: boolean };
	epic_login_code: { args: { code: string }; result: void };
	epic_login_webview: { args: void; result: void };
	epic_logout: { args: void; result: void };
	epic_auth_url: { args: void; result: string };
	epic_session_restore: { args: void; result: boolean };
};

export type RustCommandName = keyof RustCommandMap;
export type RustCommandArgs<T extends RustCommandName> = RustCommandMap[T]['args'];
export type RustCommandResult<T extends RustCommandName> = RustCommandMap[T]['result'];

export type RustCommandArgsInput<T extends RustCommandName> = RustCommandArgs<T> extends void
	? void | undefined
	: RustCommandArgs<T>;
