import { rustInvoke } from "$lib/infra/rust/invoke";
import { revealItemInDir } from "@tauri-apps/plugin-opener";

export async function downloadBepInExToCache(url: string, architecture: "x86" | "x64") {
  const cachePath = await rustInvoke("core_get_bepinex_cache_path_for_arch", { architecture });
  await rustInvoke("modding_bepinex_cache_download", { url, cachePath });
}

export async function clearBepInExCache(architecture: "x86" | "x64") {
  const cachePath = await rustInvoke("core_get_bepinex_cache_path_for_arch", { architecture });
  await rustInvoke("modding_bepinex_cache_clear", { cachePath });
}

export function autoDetectBepInExArchitecture(gamePath: string) {
  return rustInvoke("core_auto_detect_bepinex_architecture", { gamePath });
}

export function detectAmongUsPath() {
  return rustInvoke("platform_detect_among_us");
}

export function detectGameStore(path: string) {
  return rustInvoke("platform_detect_game_store", { path });
}

export async function openDataFolder() {
  const appDataPath = await rustInvoke("core_get_app_data_dir");
  await revealItemInDir(appDataPath);
}
