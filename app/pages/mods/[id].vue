<template>
	<div class="relative min-h-screen">
		<div class="absolute inset-0 bg-cover bg-center" style="background-image: url('/img/bg.jpg');" />
		<div class="absolute inset-0 bg-black/50" />

		<div class="page-content relative flex flex-col gap-5 text-white px-4 pt-20 md:pt-30">
			<div class="flex flex-col justify-center md:flex-row gap-6 w-full max-w-6xl mx-auto">
				<!-- Show a loading bar while trying to find the mod -->
				<div v-if="pending" class="flex p-8">
					<UIcon name="mingcute:loading-fill" class="size-10 animate-spin" />
				</div>

				<!-- If mod is missing or there's another error, just show this page -->
				<div v-else-if="error" class="p-4 flex flex-col items-center gap-2">
					<span class="font-semibold text-2xl text-white">Could not find mod with ID: {{ route.params.id }}</span>
					<span class="text-base text-white/70">{{ error }}</span>
					<UButton
						label="Back to Explore"
						icon="proicons:cube"
						to="/explore"
						class="px-4 py-2 text-xl mt-3 text-white bg-primary/40 backdrop-blur-md rounded-lg ring-1 ring-primary/20 hover:bg-primary/30 transform hover:scale-105 transition"
					/>
				</div>

				<div v-else>
					<h1>{{ mod.mod_name }}</h1>
					<p>{{ mod.description }}</p>
				</div>
			</div>
		</div>
	</div>
</template>

<script setup lang="ts">
	import type { Mod } from "~/types";
	import { useRoute } from "vue-router";

	const route = useRoute();
	const modId = route.params.id as string;

	const config = useRuntimeConfig();
	const { data: mod, pending, error } = await useLazyFetch<Mod[]>(`/api/v1/mods/${modId}`, {
		baseURL: config.public.apiUrl,
		default: () => [],
		retry: 3,
		server: false,
		retryDelay: 1000,
		timeout: 10000,
		onRequestError({ error }) {
			console.error("Request failed:", error);
		},
		onResponseError({ response }) {
			console.error("Response error:", response.status, response.statusText);
		}
	});
</script>
