<template>
	<div class="w-full flex-1 bg-white/10 p-4 rounded-lg transition-all duration-150 border-3 border-transparent focus-within:border-white/50 focus-within:shadow-lg">
		<UInput
			v-model="searchQuery"
			class="w-full [--ui-text-dimmed:white] [--ui-text-muted:white]"
			input-class="w-full bg-transparent"
			size="xl"
			variant="none"
			placeholder="Search mods"
			icon="i-lucide-search"
			style="
        --ui-text-dimmed: white;
        --ui-text-muted: white;
        color: white;"
		/>
	</div>

	<div v-if="pending" class="flex justify-center p-8">
		<UIcon
			name="mingcute:loading-fill"
			class="size-10 animate-spin"
		/>
	</div>

	<div v-else-if="error" class="flex flex-col items-center text-center p-4 gap-2">
		<span class="font-semibold text-2xl">Failed to load mods. Try refreshing the page.</span>
		<span class="text-base text-white/60">{{ error }}</span>
	</div>

	<div v-else-if="filteredMods.length === 0" class="flex items-center justify-center text-center p-4 gap-2">
		<UIcon
			name="material-symbols:sad-tab"
			class="size-10"
		/>
		<span class="font-semibold">No results found</span>
	</div>

	<template v-else>
		<li v-for="(mod, index) in paginatedMods" :key="mod.mod_id || index">
			<ModCard
				:mod="mod"
			/>
		</li>

		<UPagination
			v-if="filteredMods.length > pageSize"
			v-model:page="page"
			:items-per-page="pageSize"
			:total="filteredMods.length"
			size="xl"
			variant="ghost"
			active-variant="solid"
			class="[&_button]:cursor-pointer flex items-center justify-center text-center mt-5"
		/>
	</template>
</template>

<script setup lang="ts">
	import type { Mod } from "~/types";
	import ModCard from "~/components/modspage/ModCard.vue";

	const page = ref(1);
	const pageSize = ref(4);
	const config = useRuntimeConfig();
	const searchQuery = ref<string>("");

	const { data: mods, pending, error } = await useLazyFetch<Mod[]>("/api/v1/mods", {
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

	const filteredMods = computed(() => {
		if (!searchQuery.value || !mods.value) return mods.value || [];
		const query = searchQuery.value.toLowerCase();
		return mods.value.filter((mod) =>
			mod.mod_name?.toLowerCase().includes(query)
			|| mod.description?.toLowerCase().includes(query)
			|| mod.author?.toLowerCase().includes(query)
		);
	});

	const paginatedMods = computed(() => {
		if (!filteredMods.value) return [];

		const start = (page.value - 1) * pageSize.value;
		const end = start + pageSize.value;
		return filteredMods.value.slice(start, end);
	});

	watch(searchQuery, () => {
		page.value = 1;
	});
</script>
