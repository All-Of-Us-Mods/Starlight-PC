import { setContext, getContext, type Snippet } from 'svelte';

class SidebarState {
	#content = $state<Snippet | null>(null);
	#isOpen = $state(false);
	#isMaximized = $state(false);
	#contentId = $state<string | null>(null);
	#onCloseCallback: (() => void) | null = null;

	get content() {
		return this.#content;
	}
	get isOpen() {
		return this.#isOpen;
	}
	get isMaximized() {
		return this.#isMaximized;
	}
	get contentId() {
		return this.#contentId;
	}

	/**
	 * Opens the sidebar with content. If an `id` is provided and matches the
	 * currently open content, the sidebar will close instead (toggle behavior).
	 * @returns `true` if opened, `false` if closed (toggled off)
	 */
	open(content: Snippet, onClose?: () => void, id?: string): boolean {
		// Toggle behavior: if same id is already open, close instead
		if (id && this.#contentId === id && this.#isOpen) {
			this.close();
			return false;
		}

		// Call previous callback when replacing content while open
		if (this.#isOpen && this.#onCloseCallback) {
			this.#onCloseCallback();
		}

		this.#content = content;
		this.#isOpen = true;
		this.#contentId = id ?? null;
		this.#onCloseCallback = onClose ?? null;
		return true;
	}

	close() {
		this.#isOpen = false;
	}

	toggleMaximize() {
		this.#isMaximized = !this.#isMaximized;
	}

	finalizeClose() {
		if (!this.#isOpen) {
			this.#content = null;
			this.#isMaximized = false;
			this.#contentId = null;
			this.#onCloseCallback?.();
			this.#onCloseCallback = null;
		}
	}
}

const SIDEBAR_KEY = Symbol('sidebar');

export function setSidebar() {
	const sidebar = new SidebarState();
	setContext(SIDEBAR_KEY, sidebar);
	return sidebar;
}

export function getSidebar() {
	const context = getContext<SidebarState>(SIDEBAR_KEY);
	if (!context) {
		throw new Error('getSidebar must be used within a sidebar context provider');
	}
	return context;
}

/**
 * Automatically opens the sidebar with the given snippet when the
 * component mounts, and closes it when the component unmounts.
 */
export function useSidebar(content: Snippet) {
	const sidebar = getSidebar();
	if (!sidebar) return;

	$effect(() => {
		sidebar.open(content);
		return () => sidebar.close();
	});
}
