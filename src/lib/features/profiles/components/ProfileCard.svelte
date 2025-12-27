<script lang="ts">
	import * as Card from '$lib/components/ui/card';
	import { Button } from '$lib/components/ui/button';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
	import { Play, FolderOpen, Trash2, Calendar, Package, EllipsisVertical } from '@lucide/svelte';
	import { launchService } from '../launch-service';
	import { open } from '@tauri-apps/plugin-shell';
	import { profileService } from '../profile-service';
	import type { Profile } from '../schema';

	let { profile }: { profile: Profile } = $props();

	let isDeleting = $state(false);

	async function handleLaunch() {
		try {
			await launchService.launchProfile(profile);
		} catch (error) {
			alert(error);
		}
	}

	async function handleOpenFolder() {
		await open(profile.path, 'file');
	}

	async function handleDelete() {
		if (!confirm('Delete this profile?')) return;

		isDeleting = true;
		try {
			await profileService.deleteProfile(profile.id);
		} catch (error) {
			alert(error);
		} finally {
			isDeleting = false;
		}
	}

	const lastLaunched = $derived(
		profile.last_launched_at ? new Date(profile.last_launched_at).toLocaleDateString() : 'Never'
	);
</script>

<Card.Root class="overflow-hidden transition-colors hover:bg-accent/50">
	<Card.Content class="p-4">
		<div class="flex items-start justify-between gap-4">
			<div class="flex min-w-0 flex-1 flex-col gap-1">
				<h3 class="truncate text-lg font-bold" title={profile.name}>
					{profile.name}
				</h3>
				<div class="flex items-center gap-4 text-sm text-muted-foreground">
					<div class="flex items-center gap-1.5">
						<Package class="h-4 w-4" />
						<span>{profile.mods.length} mods</span>
					</div>
					<div class="flex items-center gap-1.5">
						<Calendar class="h-4 w-4" />
						<span>{lastLaunched}</span>
					</div>
				</div>
			</div>

			<div class="flex items-center gap-2">
				<Button size="sm" onclick={handleLaunch} disabled={isDeleting}>
					<Play class="mr-2 h-4 w-4 fill-current" />
					Launch
				</Button>

				<DropdownMenu.Root>
					<DropdownMenu.Trigger>
						{#snippet child({ props })}
							<Button {...props} variant="ghost" size="icon" aria-label="Profile actions">
								<EllipsisVertical class="h-5 w-5" />
							</Button>
						{/snippet}
					</DropdownMenu.Trigger>
					<DropdownMenu.Content align="end">
						<DropdownMenu.Group>
							<DropdownMenu.Item onclick={handleLaunch}>
								<Play class="mr-2 h-4 w-4" />
								<span>Launch</span>
							</DropdownMenu.Item>
							<DropdownMenu.Item onclick={handleOpenFolder}>
								<FolderOpen class="mr-2 h-4 w-4" />
								<span>Open Folder</span>
							</DropdownMenu.Item>
						</DropdownMenu.Group>
						<DropdownMenu.Separator />
						<DropdownMenu.Item
							onclick={handleDelete}
							disabled={isDeleting}
							class="text-destructive focus:bg-destructive focus:text-destructive-foreground"
						>
							<Trash2 class="mr-2 h-4 w-4" />
							<span>Delete</span>
						</DropdownMenu.Item>
					</DropdownMenu.Content>
				</DropdownMenu.Root>
			</div>
		</div>
	</Card.Content>
</Card.Root>
