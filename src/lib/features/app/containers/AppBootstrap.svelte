<script lang="ts">
	import { onMount } from 'svelte';
	import { useQueryClient } from '@tanstack/svelte-query';
	import { bootstrapApp } from '../services/bootstrap';

	const queryClient = useQueryClient();
	const noop = () => {};

	onMount(() => {
		let cleanup = noop;
		void bootstrapApp(queryClient).then((teardown) => {
			cleanup = teardown;
		});
		return () => cleanup();
	});
</script>
