<script lang="ts">
	import { toast } from 'svelte-sonner';
	import { updateState } from '$lib/features/updates/state/update-state.svelte';

	const AVAILABLE_TOAST_ID = 'update-available';
	const PROGRESS_TOAST_ID = 'update-progress';
	const ERROR_TOAST_ID = 'update-error';

	let lastError = $state<string | null>(null);

	$effect(() => {
		const updateInfo = updateState.updateInfo;
		const status = updateState.status;
		const dismissed = updateState.dismissed;

		if (updateInfo && status === 'idle' && !dismissed) {
			toast(`Update v${updateInfo.version} available`, {
				id: AVAILABLE_TOAST_ID,
				description: `Current version: v${updateInfo.currentVersion}`,
				duration: Infinity,
				action: {
					label: 'Update',
					onClick: () => void updateState.install()
				},
				onDismiss: () => updateState.dismiss()
			});
		} else {
			toast.dismiss(AVAILABLE_TOAST_ID);
		}
	});

	$effect(() => {
		const status = updateState.status;
		const percent = updateState.progress.percent;

		if (status === 'downloading') {
			toast.loading(`Downloading update... ${percent}%`, {
				id: PROGRESS_TOAST_ID,
				duration: Infinity,
				dismissable: false
			});
			return;
		}

		if (status === 'installing') {
			toast.loading('Installing update...', {
				id: PROGRESS_TOAST_ID,
				duration: Infinity,
				dismissable: false
			});
			return;
		}

		toast.dismiss(PROGRESS_TOAST_ID);
	});

	$effect(() => {
		const errorMessage = updateState.errorMessage;
		if (!errorMessage) {
			lastError = null;
			toast.dismiss(ERROR_TOAST_ID);
			return;
		}

		if (errorMessage === lastError) return;

		lastError = errorMessage;
		toast.error('Update failed', {
			id: ERROR_TOAST_ID,
			description: errorMessage,
			action: {
				label: 'Retry',
				onClick: () => void updateState.check()
			}
		});
	});
</script>
