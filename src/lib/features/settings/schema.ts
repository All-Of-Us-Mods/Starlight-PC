import { type } from "arktype";

export const Settings = type({
  bepinex_url_x86: "string",
  bepinex_url_x64: "string",
  among_us_path: "string",
  close_on_launch: "boolean",
  allow_multi_instance_launch: "boolean",
  game_platform: "'steam' | 'epic' | 'xbox'",
  cache_bepinex: "boolean",
  "xbox_app_id?": "string",
  linux_runner_kind: "'wine' | 'proton'",
  linux_runner_binary: "string",
  linux_wine_prefix: "string",
  linux_proton_compat_data_path: "string",
  linux_proton_steam_client_path: "string",
  linux_proton_use_steam_run: "boolean",
});

export type AppSettings = typeof Settings.infer;

export type GamePlatform = AppSettings["game_platform"];
export type LinuxRunnerKind = AppSettings["linux_runner_kind"];
