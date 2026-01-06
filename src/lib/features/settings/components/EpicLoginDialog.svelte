<script lang="ts">
	import {
		Dialog,
		DialogContent,
		DialogDescription,
		DialogHeader,
		DialogTitle
	} from '$lib/components/ui/dialog';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { Label } from '$lib/components/ui/label';
	import { showError, showSuccess } from '$lib/utils/toast';
	import { epicService } from '../epic-service';
	import { openUrl } from '@tauri-apps/plugin-opener';
	import { LogOut, ExternalLink, LoaderCircle, ChevronDown, ChevronUp } from '@lucide/svelte';
	import { onDestroy } from 'svelte';

	interface Props {
		open?: boolean;
		onChange?: () => void;
	}

	let { open = $bindable(), onChange }: Props = $props();

	let isLoggedIn = $state(false);
	let isWebviewOpen = $state(false);
	let showManualFallback = $state(false);
	let authCode = $state('');
	let isLoading = $state(false);

	let unsubscribe: (() => void) | null = null;

	$effect(() => {
		if (open) {
			epicService.isLoggedIn().then((v) => (isLoggedIn = v));
			unsubscribe = epicService.subscribeToLoginEvents({
				onStarted: () => (isWebviewOpen = true),
				onSuccess: () => handleSuccess(),
				onError: (e) => handleError(e),
				onCancelled: () => (isWebviewOpen = false)
			});
		} else {
			reset();
		}
	});

	onDestroy(() => unsubscribe?.());

	function reset() {
		unsubscribe?.();
		unsubscribe = null;
		isWebviewOpen = false;
		showManualFallback = false;
		authCode = '';
		isLoading = false;
	}

	function handleSuccess() {
		isWebviewOpen = false;
		isLoggedIn = true;
		isLoading = false;
		showSuccess('Logged into Epic Games');
		open = false;
		onChange?.();
	}

	function handleError(error: string) {
		isWebviewOpen = false;
		isLoading = false;
		showError(error);
	}

	async function loginWithWebview() {
		try {
			await epicService.loginWithWebview();
		} catch (e) {
			handleError(String(e));
		}
	}

	async function loginWithCode() {
		if (!authCode.trim()) return;
		isLoading = true;
		try {
			await epicService.login(authCode.trim());
			handleSuccess();
		} catch (e) {
			handleError(String(e));
		}
	}

	async function logout() {
		isLoading = true;
		try {
			await epicService.logout();
			showSuccess('Logged out of Epic Games');
			isLoggedIn = false;
			onChange?.();
		} catch (e) {
			showError(e);
		} finally {
			isLoading = false;
		}
	}

	async function openAuthPage() {
		try {
			await openUrl(await epicService.getAuthUrl());
		} catch (e) {
			showError(e);
		}
	}
</script>

<Dialog bind:open>
	<DialogContent>
		<DialogHeader>
			<DialogTitle>Epic Games Login</DialogTitle>
			<DialogDescription>
				{#if isLoggedIn}
					Your Epic Games account is connected.
				{:else if isWebviewOpen}
					Complete the login in the Epic Games window.
				{:else}
					Login to launch the Epic Games version of Among Us.
				{/if}
			</DialogDescription>
		</DialogHeader>

		<div class="space-y-4 py-4">
			{#if isLoggedIn}
				<div class="flex items-center justify-between rounded-lg bg-muted/50 p-4">
					<div>
						<p class="text-sm font-medium">Logged In</p>
						<p class="text-sm text-muted-foreground">Your session is active</p>
					</div>
					<Button variant="outline" onclick={logout} disabled={isLoading} class="gap-2">
						{#if isLoading}
							<LoaderCircle class="h-4 w-4 animate-spin" />
						{:else}
							<LogOut class="h-4 w-4" />
						{/if}
						Logout
					</Button>
				</div>
			{:else if isWebviewOpen}
				<div class="flex flex-col items-center gap-4 py-8">
					<LoaderCircle class="h-8 w-8 animate-spin text-primary" />
					<p class="text-sm text-muted-foreground">Waiting for login...</p>
				</div>
				<div class="flex justify-center">
					<Button variant="outline" onclick={() => (open = false)}>Cancel</Button>
				</div>
			{:else}
				<Button onclick={loginWithWebview} class="w-full" size="lg">Login to Epic Games</Button>

				<button
					type="button"
					class="flex w-full items-center justify-center gap-1 pt-2 text-sm text-muted-foreground hover:text-foreground"
					onclick={() => (showManualFallback = !showManualFallback)}
				>
					{#if showManualFallback}
						<ChevronUp class="h-4 w-4" />
					{:else}
						<ChevronDown class="h-4 w-4" />
					{/if}
					Having trouble? Use manual login
				</button>

				{#if showManualFallback}
					<div class="space-y-4 rounded-lg border p-4">
						<div class="space-y-2">
							<Label>Step 1: Open Epic Games in browser</Label>
							<Button variant="outline" onclick={openAuthPage} class="w-full gap-2">
								<ExternalLink class="h-4 w-4" />
								Open Login Page
							</Button>
						</div>

						<div class="space-y-2">
							<Label for="auth-code">Step 2: Enter Authorization Code</Label>
							<Input
								id="auth-code"
								bind:value={authCode}
								placeholder="Paste authorizationCode here"
								onkeydown={(e) => e.key === 'Enter' && loginWithCode()}
							/>
							<p class="text-xs text-muted-foreground">
								Copy the <code class="rounded bg-muted px-1">"authorizationCode"</code> value from the
								page.
							</p>
						</div>

						<div class="flex justify-end">
							<Button onclick={loginWithCode} disabled={isLoading || !authCode.trim()}>
								{#if isLoading}
									<LoaderCircle class="mr-2 h-4 w-4 animate-spin" />
								{/if}
								Login
							</Button>
						</div>
					</div>
				{/if}
			{/if}
		</div>
	</DialogContent>
</Dialog>
