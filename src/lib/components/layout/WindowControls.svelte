<script lang="ts">
	import type { TauriWindow } from './types';

	let { appWindow, platformName }: { appWindow: TauriWindow | null; platformName: string } =
		$props();
</script>

{#if platformName === 'windows'}
	<div class="window-controls">
		<button aria-label="Minimize" onclick={() => appWindow?.minimize()}>
			<svg width="10" height="1"><path d="M0 0h10v1H0z" fill="currentColor" /></svg>
		</button>
		<button aria-label="Maximize" onclick={() => appWindow?.toggleMaximize()}>
			<svg width="10" height="10">
				<path d="M0 0v10h10V0H0zm9 1v8H1V1h8z" fill="currentColor" />
			</svg>
		</button>
		<button aria-label="Close" onclick={() => appWindow?.close()} class="close">
			<svg width="10" height="10">
				<path d="M0 0l10 10M10 0L0 10" stroke="currentColor" fill="none" />
			</svg>
		</button>
	</div>
{/if}

<style lang="postcss">
	@reference "$lib/../app.css";

	.window-controls {
		@apply flex h-full;

		button {
			@apply flex h-full w-[45px] items-center justify-center transition-colors hover:bg-white/10;
			-webkit-app-region: no-drag;

			&.close:hover {
				@apply bg-red-500;
			}
		}
	}
</style>
