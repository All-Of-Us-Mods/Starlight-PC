<template>
	<NuxtLink to="/mods" class="home-card">
		<!-- fixed width -->
		<img :src="mod.image" alt="mod image" class="w-16 h-16 rounded-lg object-contain">

		<div class="flex flex-col">
			<h3 class="text-lg font-bold text-white break-words">
				{{ mod.title }}
			</h3>
			<p class="text-white/80 mt-1 text-sm break-words">
				{{ mod.description }}
			</p>
			<p class="text-success mt-2 flex items-center gap-1">
				<UIcon
					name="material-symbols:download"
					class="size-5"
				/>
				{{ formatDownloads(mod.downloads) }}
			</p>
		</div>
	</NuxtLink>
</template>

<script setup lang="ts">
	defineProps<{
		mod: {
			image: string
			title: string
			description: string
			downloads: number
		}
	}>();

	function formatDownloads(value: number) {
		if (value >= 1_000_000) return `${(value / 1_000_000).toFixed(1)}M`;
		if (value >= 1_000) return `${(value / 1_000).toFixed(1)}k`;
		return value.toString();
	}
</script>
