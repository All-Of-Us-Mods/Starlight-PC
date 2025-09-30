<template>
	<div class="relative w-7xl mx-auto mt-5">
		<div class="absolute inset-y-0 left-0 w-20 bg-gradient-to-r from-black/30 to-transparent pointer-events-none z-20" />
		<div class="absolute inset-y-0 right-0 w-20 bg-gradient-to-l from-black/30 to-transparent pointer-events-none z-20" />

		<ClientOnly>
			<UCarousel
				class="overflow-hidden"
				:items="mods"
				loop
				:auto-scroll="{
					speed: props.speed ?? 1
				}"
				:watch-drag="false"
				:ui="{ item: 'basis-1/3' }"
			>
				<template #default="{ item }">
					<ModCard
						:mod="item"
					/>
				</template>
			</UCarousel>
		</ClientOnly>
	</div>
</template>

<script setup lang="ts">
	import { defineProps } from "vue";
	import ModCard from "~/components/homepage/HomeModCard.vue";

	const props = defineProps<{
		speed?: number
	}>();

	const mods = [
		{
			image: "/img/mira.png",
			title: "Town Of Us Mira",
			description: "An Among Us mods based on Town of Salem, designed to be a rewrite of Town of Us Reactivated!",
			downloads: 5000
		},
		{
			image: "/img/launchpad.png",
			title: "Launchpad: Reloaded",
			description: "Launchpad Reloaded is a free Among Us mods that adds a ton of new roles, gamemodes, and features to the game (such as gradient colors, and new voting types!)",
			downloads: 100000
		},
		{
			image: "/img/newmod.png",
			title: "NewMod",
			description: "NewMod is a mods for Among Us that adds new roles, modifiers, features, and game modes",
			downloads: 5000
		},
		{
			image: "/img/ehr.png",
			title: "Endless Host Roles",
			description: "An Among Us mods meant to add content whilst keeping the vanilla feel!",
			downloads: 5000
		}
	];
</script>
