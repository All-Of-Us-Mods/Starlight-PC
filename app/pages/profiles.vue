<template>
	<div class="relative min-h-screen">
		<div class="absolute inset-0 bg-cover bg-center" style="background-image: url('/img/bg.jpg');" />
		<div class="absolute inset-0 bg-black/50" />

		<div class="page-content relative flex flex-col gap-5 text-white px-4 pt-20 md:pt-30">
			<div class="mb-8 rounded-xl border border-white/20 bg-white/10 p-6 backdrop-blur-sm">
				<h1 class="text-3xl font-semibold">
					Profiles
				</h1>
				<p class="mt-2 text-white/70">
					Create and manage local profiles for different mod setups.
				</p>
			</div>

			<div class="mb-8 rounded-xl border border-white/20 bg-white/10 p-6 backdrop-blur-sm">
				<label class="block text-sm font-medium text-white/80" for="profile-name">Profile name</label>
				<div class="mt-2 flex flex-col gap-3 md:flex-row md:items-center">
					<UInput
						id="profile-name"
						v-model="newProfileName"
						placeholder="e.g. Stream setup"
						:disabled="isSubmitting"
						class="md:flex-1"
						@keyup.enter="handleCreate"
					/>
					<UButton
						label="Add profile"
						color="primary"
						:loading="isSubmitting"
						:disabled="isSubmitting"
						@click="handleCreate"
					/>
				</div>
				<div v-if="feedback" class="mt-4">
					<UAlert
						:title="feedback.message"
						:color="feedback.kind === 'error' ? 'error' : 'success'"
						variant="soft"
						icon="i-heroicons-information-circle"
					/>
				</div>
			</div>

			<div class="flex-1 space-y-4 pb-10">
				<div v-if="!profiles.length" class="rounded-xl border border-dashed border-white/30 bg-white/5 p-8 text-center text-white/70">
					No profiles yet. Create one to get started.
				</div>

				<ul v-else class="space-y-4">
					<li
						v-for="profile in profiles"
						:key="profile.id"
						class="rounded-xl border bg-white/10 p-5 backdrop-blur-sm transition"
						:class="profile.id === activeProfileId ? 'border-primary/60 ring-1 ring-primary/50' : 'border-white/15'"
					>
						<div class="flex flex-col justify-between gap-3 md:flex-row md:items-center">
							<div>
								<p class="text-xl font-semibold">
									{{ profile.name }}
								</p>
								<p class="text-sm text-white/70">
									{{ profile.path }}
								</p>
								<p class="text-xs text-white/60">
									Created {{ formatTimestamp(profile.created_at) }}
								</p>
								<UButton
									label="Launch with this profile"
									color="primary"
									size="sm"
									class="mt-2"
									:loading="launchingProfileId === profile.id"
									:disabled="launchingProfileId === profile.id"
									@click="handleLaunch(profile)"
								/>
							</div>
							<span
								v-if="profile.id === activeProfileId"
								class="inline-flex items-center rounded-full bg-primary/20 px-3 py-1 text-xs font-medium text-primary"
							>
								Active
							</span>
						</div>
					</li>
				</ul>
			</div>
		</div>
	</div>
</template>

<script setup lang="ts">
	import type { ProfileEntry } from "~/types";
	import { useApp } from "~/composables/useApp";

	interface Feedback {
		kind: "success" | "error"
		message: string
	}

	const { initApp, storeData, createProfile, launchGame } = useApp();

	const newProfileName = ref("");
	const isSubmitting = ref(false);
	const feedback = ref<Feedback | null>(null);
	const launchingProfileId = ref<string | null>(null);

	onMounted(async () => {
		await initApp();
	});

	const profiles = computed<ProfileEntry[]>(() => (storeData.value.profiles ?? []) as ProfileEntry[]);
	const activeProfileId = computed(() => storeData.value.active_profile as string | null);

	const handleLaunch = async (profile: ProfileEntry) => {
		launchingProfileId.value = profile.id;
		try {
			await launchGame(profile.id);
			feedback.value = {
				kind: "success",
				message: `Launching Among Us with "${profile.name}"...`
			};
		} catch (error) {
			const message = typeof error === "string" ? error : error instanceof Error ? error.message : "Failed to launch Among Us.";
			feedback.value = {
				kind: "error",
				message
			};
		} finally {
			launchingProfileId.value = null;
		}
	};

	const handleCreate = async () => {
		const name = newProfileName.value.trim();
		if (!name) {
			feedback.value = {
				kind: "error",
				message: "Please enter a profile name."
			};
			return;
		}

		isSubmitting.value = true;
		feedback.value = null;

		try {
			const profile = await createProfile(name);
			newProfileName.value = "";
			feedback.value = {
				kind: "success",
				message: `Created profile "${profile.name}".`
			};
		} catch (error) {
			const message = typeof error === "string" ? error : error instanceof Error ? error.message : "Failed to create profile.";
			feedback.value = {
				kind: "error",
				message
			};
		} finally {
			isSubmitting.value = false;
		}
	};

	const formatTimestamp = (seconds: number) => {
		if (!Number.isFinite(seconds)) return "Unknown";
		const date = new Date(seconds * 1000);
		return date.toLocaleString();
	};
</script>
