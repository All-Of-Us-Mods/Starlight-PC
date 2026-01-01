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
	import { showToastError, showToastSuccess } from '$lib/utils/toast';
	import { epicService } from '../epic-service';
	import { openUrl } from '@tauri-apps/plugin-opener';
	import { LogOut, ExternalLink } from '@lucide/svelte';

	interface Props {
		open?: boolean;
		onChange?: () => void;
	}

	let { open = $bindable(), onChange }: Props = $props();

	let isLoggedIn = $state(false);

	$effect(() => {
		if (open) {
			epicService.isLoggedIn().then((v) => (isLoggedIn = v));
		}
	});

	let authCode = $state('');
	let isLoggingIn = $state(false);
	let isLoggingOut = $state(false);

	async function handleLogin() {
		if (!authCode.trim()) {
			showToastError('Please enter the authorization code');
			return;
		}

		isLoggingIn = true;
		try {
			await epicService.login(authCode.trim());
			showToastSuccess('Successfully logged into Epic Games');
			authCode = '';
			open = false;
			isLoggedIn = true;
			onChange?.();
		} catch (e) {
			showToastError(e);
		} finally {
			isLoggingIn = false;
		}
	}

	async function handleOpenBrowser() {
		try {
			const url = await epicService.getAuthUrl();
			await openUrl(url);
		} catch (e) {
			showToastError(e);
		}
	}

	async function handleLogout() {
		isLoggingOut = true;
		try {
			await epicService.logout();
			showToastSuccess('Logged out of Epic Games');
			isLoggedIn = false;
			onChange?.();
		} catch (e) {
			showToastError(e);
		} finally {
			isLoggingOut = false;
		}
	}
</script>

<Dialog bind:open>
	<DialogContent>
		<DialogHeader>
			<DialogTitle>Epic Games Login</DialogTitle>
			{#if isLoggedIn}
				<DialogDescription>
					You are currently logged into Epic Games. Launching Epic Games version will use your
					account automatically.
				</DialogDescription>
			{:else}
				<DialogDescription>
					Login to your Epic Games account to launch the Epic Games version of Among Us.
				</DialogDescription>
			{/if}
		</DialogHeader>

		<div class="space-y-4 py-4">
			{#if isLoggedIn}
				<div class="flex items-center justify-between rounded-lg bg-muted/50 p-4">
					<div>
						<p class="text-sm font-medium">Logged In</p>
						<p class="text-sm text-muted-foreground">Your session is active</p>
					</div>
					<Button variant="outline" onclick={handleLogout} disabled={isLoggingOut} class="gap-2">
						{#if isLoggingOut}
							<div
								class="h-4 w-4 animate-spin rounded-full border-2 border-primary border-t-transparent"
							></div>
						{:else}
							<LogOut class="h-4 w-4" />
						{/if}
						Logout
					</Button>
				</div>
			{:else}
				<div class="space-y-2">
					<Label for="auth-url">Step 1: Login to Epic Games</Label>
					<Button onclick={handleOpenBrowser} class="w-full gap-2">
						<ExternalLink class="h-4 w-4" />
						Open Epic Games Login Page
					</Button>
					<p class="text-sm text-muted-foreground">
						This will open your browser. After logging in, you'll be redirected to a page with a
						code in the URL.
					</p>
				</div>

				<div class="space-y-2">
					<Label for="auth-code">Step 2: Enter Authorization Code</Label>
					<Input
						id="auth-code"
						bind:value={authCode}
						placeholder="e.g., u3rhcmxpz2h0ifbdielziedvyxrlzce"
						onkeydown={(e) => {
							if (e.key === 'Enter') {
								handleLogin();
							}
						}}
					/>
					<p class="text-sm text-muted-foreground">
						Copy code from the redirected URL and paste it here. The code is the long string after <code
							class="rounded bg-muted px-1 py-0.5 text-xs">"authorizationCode":</code
						>.
					</p>
				</div>
			{/if}
		</div>

		{#if !isLoggedIn}
			<div class="flex justify-end gap-2">
				<Button variant="outline" onclick={() => (open = false)}>Cancel</Button>
				<Button onclick={handleLogin} disabled={isLoggingIn || !authCode.trim()}>
					{#if isLoggingIn}
						<div
							class="mr-2 h-4 w-4 animate-spin rounded-full border-2 border-primary border-t-transparent"
						></div>
					{/if}
					Login
				</Button>
			</div>
		{/if}
	</DialogContent>
</Dialog>
