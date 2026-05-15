<script lang="ts" module>
	import { cn, type WithElementRef } from '$lib/utils.js';
	import type { HTMLAnchorAttributes, HTMLButtonAttributes } from 'svelte/elements';
	import { type VariantProps, tv } from 'tailwind-variants';

	export const buttonVariants = tv({
		base: "focus-visible:border-ring focus-visible:ring-ring/50 aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 aria-invalid:border-destructive inline-flex shrink-0 items-center justify-center gap-2 whitespace-nowrap rounded-md text-sm font-medium outline-none transition-all focus-visible:ring-[3px] disabled:pointer-events-none disabled:opacity-50 aria-disabled:pointer-events-none aria-disabled:opacity-50 [&_svg:not([class*='size-'])]:size-4 [&_svg]:pointer-events-none [&_svg]:shrink-0",
		variants: {
			variant: {
				default: 'bg-primary text-primary-foreground shadow-xs hover:bg-primary/90',
				destructive:
					'bg-destructive shadow-xs hover:bg-destructive/90 focus-visible:ring-destructive/20 dark:focus-visible:ring-destructive/40 dark:bg-destructive/60 text-white',
				outline:
					'bg-background shadow-xs hover:bg-accent hover:text-accent-foreground dark:bg-input/30 dark:border-input dark:hover:bg-input/50 border',
				secondary: 'bg-secondary text-secondary-foreground shadow-xs hover:bg-secondary/80',
				ghost: 'hover:bg-accent hover:text-accent-foreground dark:hover:bg-accent/50',
				link: 'text-primary underline-offset-4 hover:underline',
				titlebar:
					'titlebar-button relative h-full w-12 bg-transparent text-foreground transition-colors last:w-[3.75rem] last:pr-3 [&_svg]:h-5 [&_svg]:w-5 [&_svg]:relative [&_svg]:z-10',
				'titlebar-close':
					'titlebar-button titlebar-button-close relative h-full w-12 bg-transparent text-foreground transition-colors last:w-[3.75rem] last:pr-3 [&_svg]:h-5 [&_svg]:w-5 [&_svg]:relative [&_svg]:z-10',
				navigation:
					'rounded-full bg-accent text-foreground hover:brightness-75 active:brightness-90 transition-all [&_svg]:h-5 [&_svg]:w-5'
			},
			size: {
				default: 'h-9 px-4 py-2 has-[>svg]:px-3',
				sm: 'h-8 gap-1.5 rounded-md px-3 has-[>svg]:px-2.5',
				lg: 'h-10 rounded-md px-6 has-[>svg]:px-4',
				icon: 'size-9',
				'icon-sm': 'size-8',
				'icon-lg': 'size-10'
			}
		},
		compoundVariants: [
			{
				variant: 'navigation',
				class: 'p-0 size-8 aspect-square rounded-full'
			}
		],
		defaultVariants: {
			variant: 'default',
			size: 'default'
		}
	});

	export type ButtonVariant = VariantProps<typeof buttonVariants>['variant'];
	export type ButtonSize = VariantProps<typeof buttonVariants>['size'];

	export type ButtonProps = WithElementRef<HTMLButtonAttributes> &
		WithElementRef<HTMLAnchorAttributes> & {
			variant?: ButtonVariant;
			size?: ButtonSize;
		};
</script>

<script lang="ts">
	let {
		class: className,
		variant = 'default',
		size = 'default',
		ref = $bindable(null),
		href = undefined,
		type = 'button',
		disabled,
		children,
		...restProps
	}: ButtonProps = $props();
</script>

{#if href}
	<a
		bind:this={ref}
		data-slot="button"
		class={cn(buttonVariants({ variant, size }), className)}
		href={disabled ? undefined : href}
		aria-disabled={disabled}
		role={disabled ? 'link' : undefined}
		tabindex={disabled ? -1 : undefined}
		{...restProps}
	>
		{@render children?.()}
	</a>
{:else}
	<button
		bind:this={ref}
		data-slot="button"
		class={cn(buttonVariants({ variant, size }), className)}
		{type}
		{disabled}
		{...restProps}
	>
		{@render children?.()}
	</button>
{/if}

<style>
	.titlebar-button {
		user-select: none;
		-webkit-user-select: none;
	}

	.titlebar-button::before {
		content: '';
		position: absolute;
		width: 2.7rem;
		height: 2.7rem;
		border-radius: 50%;
		background: transparent;
		transition: all 0.2s;
		z-index: 0;
		left: 50%;
		top: 50%;
		transform: translate(-50%, -50%);
	}

	.titlebar-button:hover::before {
		background: var(--accent);
		transform: translate(-50%, -50%) scale(1.11);
	}

	.titlebar-button-close:hover::before {
		background: var(--destructive);
	}

	.titlebar-button-close:hover {
		color: var(--destructive-foreground);
	}
</style>
