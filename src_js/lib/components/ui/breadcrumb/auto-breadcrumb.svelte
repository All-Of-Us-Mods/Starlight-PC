<script lang="ts">
	import { page } from '$app/state';
	import * as Breadcrumb from './index.js';
	import type { Component } from 'svelte';

	interface BreadcrumbSegment {
		label: string;
		href: string;
		isLast: boolean;
	}

	interface Props {
		/**
		 * Optional custom labels for specific routes
		 * Example: { "/settings": "Settings", "/settings/profile": "Profile" }
		 */
		labels?: Record<string, string>;
		/**
		 * Optional function to transform segment names
		 * Example: (segment) => segment.replace(/-/g, ' ')
		 */
		formatLabel?: (segment: string) => string;
		/**
		 * Maximum number of breadcrumbs to show before collapsing
		 */
		maxItems?: number;
		/**
		 * Home icon component to use for the root breadcrumb
		 */
		homeIcon?: Component;
		/**
		 * Show home as first breadcrumb
		 */
		showHome?: boolean;
		/**
		 * Custom separator component or string
		 */
		separator?: string;
	}

	let {
		labels = {},
		formatLabel = defaultFormatLabel,
		maxItems = 5,
		homeIcon,
		showHome = true,
		separator = '/'
	}: Props = $props();

	/**
	 * Default label formatter: capitalizes and replaces hyphens/underscores with spaces
	 */
	function defaultFormatLabel(segment: string): string {
		return segment
			.replace(/[-_]/g, ' ')
			.split(' ')
			.map((word) => word.charAt(0).toUpperCase() + word.slice(1))
			.join(' ');
	}

	/**
	 * Generate breadcrumb segments from current pathname
	 */
	function generateBreadcrumbs(pathname: string): BreadcrumbSegment[] {
		// Handle root path
		if (pathname === '/') {
			if (!showHome) {
				return [];
			}
			return [
				{
					label: labels['/'] || 'Home',
					href: '/',
					isLast: true
				}
			];
		}

		const segments: BreadcrumbSegment[] = [];

		// Split pathname into parts and build breadcrumbs
		const parts = pathname.split('/').filter(Boolean);
		let currentPath = '';

		parts.forEach((part, index) => {
			currentPath += `/${part}`;
			const isLast = index === parts.length - 1;

			// Check for custom label first, then use formatter
			const label = labels[currentPath] || labels[`/${part}`] || formatLabel(part);

			segments.push({
				label,
				href: currentPath,
				isLast
			});
		});

		return segments;
	}

	/**
	 * Collapse breadcrumbs if they exceed maxItems
	 */
	function collapseBreadcrumbs(
		breadcrumbs: BreadcrumbSegment[]
	): (BreadcrumbSegment & { isEllipsis?: boolean })[] {
		if (breadcrumbs.length <= maxItems) {
			return breadcrumbs;
		}

		// Always show first, last, and collapse middle items
		const first = breadcrumbs[0];
		const last = breadcrumbs[breadcrumbs.length - 1];
		const beforeLast = breadcrumbs[breadcrumbs.length - 2];

		return [first, { label: '...', href: '', isLast: false, isEllipsis: true }, beforeLast, last];
	}

	// Reactive state for breadcrumbs - derived from page pathname
	const displayBreadcrumbs = $derived.by(() => {
		const breadcrumbs = generateBreadcrumbs(page.url.pathname);
		return collapseBreadcrumbs(breadcrumbs);
	});
</script>

{#if displayBreadcrumbs.length > 0}
	<Breadcrumb.Root>
		<Breadcrumb.List>
			{#each displayBreadcrumbs as crumb, index (crumb.href || index)}
				<Breadcrumb.Item>
					{#if 'isEllipsis' in crumb && crumb.isEllipsis}
						<Breadcrumb.Ellipsis />
					{:else if crumb.isLast}
						<Breadcrumb.Page>{crumb.label}</Breadcrumb.Page>
					{:else}
						<Breadcrumb.Link href={crumb.href}>
							{#if crumb.href === '/' && homeIcon}
								{@const HomeIcon = homeIcon}
								{#if HomeIcon}
									<HomeIcon class="h-4 w-4" />
								{/if}
							{:else}
								{crumb.label}
							{/if}
						</Breadcrumb.Link>
					{/if}
				</Breadcrumb.Item>
				{#if !crumb.isLast}
					<Breadcrumb.Separator>{separator}</Breadcrumb.Separator>
				{/if}
			{/each}
		</Breadcrumb.List>
	</Breadcrumb.Root>
{/if}
