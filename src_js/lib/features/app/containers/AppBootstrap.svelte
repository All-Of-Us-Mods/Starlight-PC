<script lang="ts">
	import { onMount } from 'svelte';
	import { useQueryClient } from '@tanstack/svelte-query';
	import { bootstrapApp } from '../services/bootstrap';

	const queryClient = useQueryClient();

	onMount(() => {
		let teardown: (() => void) | undefined;
		let unmounted = false;

		void bootstrapApp(queryClient).then((cleanup) => {
			if (unmounted) {
				cleanup();
				return;
			}

			teardown = cleanup;
		});

		return () => {
			unmounted = true;
			teardown?.();
		};
	});
</script>
