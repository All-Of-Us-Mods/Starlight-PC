import { getCurrentWindow } from "@tauri-apps/api/window";
import { platform } from "@tauri-apps/plugin-os";
import type { Platform } from "$lib/components/layout/types";

export interface WindowController {
  minimize(): Promise<void>;
  toggleMaximize(): Promise<void>;
  close(): Promise<void>;
  isMaximized(): Promise<boolean>;
}

export function hasTauriWindowInternals(): boolean {
  const tauriWindow = window as Window & { __TAURI_INTERNALS__?: unknown };
  return !!tauriWindow.__TAURI_INTERNALS__;
}

export function getCurrentWindowController(): WindowController {
  return getCurrentWindow();
}

export function getWindowPlatform(): Platform {
  const os = platform();
  if (os === "macos" || os === "windows") {
    return os;
  }
  return os === "linux" ? "linux" : "other";
}

export function closeCurrentWindow(): Promise<void> {
  return getCurrentWindow().close();
}
