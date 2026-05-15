export type Platform = "macos" | "windows" | "linux" | "other";

export interface WindowController {
  minimize(): Promise<void>;
  toggleMaximize(): Promise<void>;
  close(): Promise<void>;
  isMaximized(): Promise<boolean>;
}

export interface SidebarController {
  content: import("svelte").Snippet | null;
  isOpen: boolean;
  isMaximized: boolean;
  contentId: string | null;
  open(content: import("svelte").Snippet, onClose?: () => void, id?: string): boolean;
  close(): void;
  toggleMaximize(): void;
  finalizeClose(): void;
}
