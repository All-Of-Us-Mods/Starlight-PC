<script lang="ts">
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import type { Page } from '@sveltejs/kit';
	import type { HTMLAttributes } from 'svelte/elements';
	import type { Snippet } from 'svelte';

	type RouteFunction = (page: Page) => boolean;

	export interface NavButtonProps extends Omit<HTMLAttributes<HTMLElement>, 'children'> {
		/**
		 * Navigation target - can be a URL string or a function to execute
		 */
		to: string | (() => void);
		/**
		 * Function to determine if this is the primary active route
		 */
		isPrimary?: RouteFunction;
		/**
		 * Function to determine if this is a subpage of the active route
		 */
		isSubpage?: RouteFunction;
		/**
		 * Whether the button is disabled
		 */
		disabled?: boolean;
		/**
		 * Custom class names
		 */
		class?: string;
		/**
		 * Tooltip text (for accessibility)
		 */
		tooltip?: string;
		/**
		 * Content to render inside the button
		 */
		children?: Snippet;
	}

	let {
		to,
		isPrimary,
		isSubpage,
		disabled = false,
		class: className,
		tooltip,
		children,
		...restProps
	}: NavButtonProps = $props();

	let currentPage = $derived($page);

	// Reactive computed values
	const isRouterLinkActive = $derived(isPrimary?.(currentPage) ?? false);
	const isSubpageActive = $derived(isSubpage?.(currentPage) ?? false);
	const isLink = $derived(typeof to === 'string');
	const href = $derived(isLink ? (to as string) : undefined);

	// Event handler for navigation
	function handleClick(event: MouseEvent) {
		if (disabled) {
			event.preventDefault();
			return;
		}

		if (typeof to === 'function') {
			event.preventDefault();
			to();
		} else if (typeof to === 'string') {
			event.preventDefault();
			goto(to);
		}
	}

	// Computed class names - matching Vue component styles
	const baseClasses =
		'w-12 h-12 text-primary rounded-full flex items-center justify-center text-2xl transition-all bg-transparent border-none cursor-pointer';

	const hoverClasses = 'hover:bg-accent hover:text-accent-foreground';

	const activeClasses = $derived(
		isRouterLinkActive
			? '!text-accent-foreground !bg-accent [&_svg]:drop-shadow-[0_0_0.5rem_black]'
			: isSubpageActive
				? 'text-muted-foreground bg-muted [&_svg]:drop-shadow-[0_0_0.5rem_black]'
				: ''
	);

	const disabledClasses = $derived(disabled ? 'opacity-50 cursor-not-allowed' : '');

	const combinedClasses = $derived(
		`${baseClasses} ${hoverClasses} ${activeClasses} ${disabledClasses} ${className ?? ''}`.trim()
	);

	export { className as class };
</script>

{#if isLink && href}
	<a
		{href}
		class={combinedClasses}
		role="button"
		tabindex={disabled ? -1 : 0}
		aria-disabled={disabled}
		aria-label={tooltip}
		title={tooltip}
		data-router-link-active={isRouterLinkActive}
		data-subpage-active={isSubpageActive}
		onclick={handleClick}
		{...restProps}
	>
		{@render children?.()}
	</a>
{:else}
	<button
		type="button"
		onclick={handleClick}
		{disabled}
		class={combinedClasses}
		aria-label={tooltip}
		title={tooltip}
		data-router-link-active={isRouterLinkActive}
		data-subpage-active={isSubpageActive}
		{...restProps}
	>
		{@render children?.()}
	</button>
{/if}

<style>
	/* Prevent text selection on double-click */
	a,
	button {
		user-select: none;
		-webkit-user-select: none;
	}

	/* Keyboard focus improvements */
	a:focus-visible,
	button:focus-visible {
		outline: 2px solid hsl(var(--ring));
		outline-offset: 2px;
		z-index: 10;
	}

	/* Ensure disabled state styling */
	a[aria-disabled='true'],
	button:disabled {
		pointer-events: none;
	}
</style>
