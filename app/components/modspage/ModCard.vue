<template>
	<NuxtLink :to="`/mods/${mod.mod_id}`" class="mod-card">
		<img
			:src="`${config.public.apiUrl}${mod.thumbnail}`"
			alt="mod image"
			class="w-18 h-18 rounded-xl object-contain shadow-2xl"
		>

		<div class="flex flex-col gap-1">
			<h3 class="text-lg text-white break-words">
				<b class="mr-2">{{ mod.mod_name }}</b>
				<span class="text-sm text-white/70">{{ mod.author }}</span>
			</h3>
			<div class="flex items-center gap-1 text-white/55 text-sm">
				<UIcon
					name="material-symbols:sync-outline"
					class="size-5"
				/>
				<span>Last updated {{ formatTimeAgo(mod.updated_at ?? mod.created_at) }}</span>
			</div>
			<p class="text-white/80 mt-1 text-sm break-words">
				{{ mod.description }}
			</p>
		</div>
	</NuxtLink>
</template>

<script setup lang="ts">
	import type { Mod } from "~/types";

	defineProps<{
		mod: Mod
	}>();

	const config = useRuntimeConfig();

	const formatTimeAgo = (dateInput: string | number | Date | undefined | null) => {
		if (dateInput === undefined || dateInput === null) {
			return "unknown";
		}

		let date: Date;

		if (dateInput instanceof Date) {
			date = dateInput;
		} else if (typeof dateInput === "number") {
			date = new Date(dateInput);
		} else {
			const trimmed = dateInput.trim();
			const asNumber = Number(trimmed);

			date = Number.isFinite(asNumber) ? new Date(asNumber) : new Date(trimmed);
		}

		if (Number.isNaN(date.getTime())) {
			return "unknown";
		}

		const now = new Date();
		const diffInMs = now.getTime() - date.getTime();
		const diffInDays = Math.floor(diffInMs / (1000 * 60 * 60 * 24));

		if (diffInDays === 0) {
			return "today";
		} else if (diffInDays === 1) {
			return "1 day ago";
		} else if (diffInDays < 7) {
			return `${diffInDays} days ago`;
		} else {
			return date.toLocaleDateString("en-US", {
				year: "numeric",
				month: "short",
				day: "numeric"
			});
		}
	};
</script>
