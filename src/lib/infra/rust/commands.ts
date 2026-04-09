import type { Profile, ProfileIconSelection } from "$lib/features/profiles/schema";
import type { AppSettings, GamePlatform } from "$lib/features/settings/schema";

type AppSettingsUpdate = Omit<Partial<AppSettings>, "xbox_app_id"> & {
  xbox_app_id?: string | null;
};

export type RustCommandMap = {
  core_get_settings: { args: void; result: AppSettings };
  core_update_settings: { args: { updates: AppSettingsUpdate }; result: AppSettings };
  core_get_bepinex_cache_path: { args: void; result: string };
  core_get_bepinex_cache_path_for_arch: { args: { architecture: "x86" | "x64" }; result: string };
  core_get_app_data_dir: { args: void; result: string };
  core_auto_detect_bepinex_architecture: { args: { gamePath: string }; result: string | null };

  platform_detect_among_us: { args: void; result: string | null };
  platform_detect_game_store: { args: { path: string }; result: GamePlatform };

  profiles_get_dir: { args: void; result: string };
  profiles_list: { args: void; result: Profile[] };
  profiles_get_by_id: { args: { id: string }; result: Profile | null };
  profiles_create: { args: { name: string }; result: Profile };
  profiles_install_bepinex: { args: { profileId: string }; result: void };
  profiles_delete: { args: { profileId: string }; result: void };
  profiles_rename: { args: { profileId: string; newName: string }; result: void };
  profiles_update_icon: {
    args: { profileId: string; selection: ProfileIconSelection };
    result: void;
  };
  profiles_add_mod: {
    args: { profileId: string; modId: string; version: string; file: string };
    result: void;
  };
  profiles_remove_mod: { args: { profileId: string; modId: string }; result: void };
  profiles_add_play_time: { args: { profileId: string; durationMs: number }; result: void };
  profiles_get_mod_files: { args: { profilePath: string }; result: string[] };
  profiles_delete_mod_file: { args: { profilePath: string; fileName: string }; result: void };
  profiles_get_log: { args: { profilePath: string; fileName: string }; result: string };
  profiles_read_binary_file: { args: { path: string }; result: number[] };
  profiles_export_zip: { args: { profileId: string; destination: string }; result: void };
  profiles_import_zip: { args: { zipPath: string }; result: Profile[] };
  profiles_import_mod: { args: { profileId: string; sourcePath: string }; result: string };
  profiles_create_desktop_shortcut: {
    args: { profileId: string; iconBytes?: Uint8Array | null };
    result: string;
  };
  profiles_update_last_launched: { args: { profileId: string }; result: void };

  modding_bepinex_cache_download: {
    args: { url: string; cachePath: string };
    result: void;
  };
  modding_bepinex_cache_clear: { args: { cachePath: string }; result: void };
  modding_bepinex_cache_exists: { args: { cachePath: string }; result: boolean };
  modding_mod_download: {
    args: { modId: string; url: string; destination: string; expectedChecksum?: string };
    result: void;
  };

  game_launch_modded: {
    args: {
      gameExe: string;
      profileId: string;
      profilePath?: string;
      bepinexDll: string;
      dotnetDir: string;
      coreclrPath: string;
      platform: string;
    };
    result: void;
  };
  game_launch_vanilla: { args: { gameExe: string; platform: string }; result: void };
  game_stop_profile_instances: { args: { profileId: string }; result: number };
  game_stop_all_instances: { args: void; result: number };
  game_xbox_get_app_id: { args: void; result: string };
  game_xbox_prepare_launch: { args: { gameDir: string; profilePath: string }; result: void };
  game_xbox_launch: { args: { appId: string; profileId: string | null }; result: void };
  game_xbox_cleanup: { args: { gameDir: string }; result: void };

  epic_is_logged_in: { args: void; result: boolean };
  epic_login_code: { args: { code: string }; result: void };
  epic_login_webview: { args: void; result: void };
  epic_logout: { args: void; result: void };
  epic_auth_url: { args: void; result: string };
  epic_session_restore: { args: void; result: boolean };
};

export type RustCommandName = keyof RustCommandMap;
export type RustCommandArgs<T extends RustCommandName> = RustCommandMap[T]["args"];
export type RustCommandResult<T extends RustCommandName> = RustCommandMap[T]["result"];

type WithoutReservedInvokeArgKey<T> =
  T extends Record<string, unknown> ? ("args" extends keyof T ? never : T) : T;

export type RustCommandArgsInput<T extends RustCommandName> =
  RustCommandArgs<T> extends void
    ? void | undefined
    : WithoutReservedInvokeArgKey<RustCommandArgs<T>>;
