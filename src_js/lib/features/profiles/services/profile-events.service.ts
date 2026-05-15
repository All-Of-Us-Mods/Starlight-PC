import type { BepInExProgress, ModDownloadProgress } from "../schema";
import { gameState } from "../state/game-state.svelte";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export function listenForBepInExProgress(profileId: string): Promise<UnlistenFn> {
  return listen<BepInExProgress>("bepinex-progress", (event) => {
    if (event.payload.targetType !== "profile" || event.payload.targetId !== profileId) return;
    gameState.setBepInExProgress(profileId, event.payload);
  });
}

export function listenForModDownloadProgress(): Promise<UnlistenFn> {
  return listen<ModDownloadProgress>("mod-download-progress", (event) => {
    gameState.setModDownloadProgress(event.payload.mod_id, event.payload);
  });
}
