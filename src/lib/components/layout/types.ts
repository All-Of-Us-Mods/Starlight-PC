import type { getCurrentWindow } from '@tauri-apps/api/window';

export type TauriWindow = Awaited<ReturnType<typeof getCurrentWindow>>;
export type Platform = 'macos' | 'windows' | 'linux' | 'other';
