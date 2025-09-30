<template>
	<div class="relative min-h-screen">
		<div class="absolute inset-0 bg-cover bg-center" style="background-image: url('/img/bg.jpg');" />
		<div class="absolute inset-0 bg-black/50" />

		<div class="page-content">
			<div class="relative flex flex-col items-center gap-5 text-white px-4 pt-20 md:pt-40">
				<h1 class="font-bold text-7xl text-white text-center">
					Among Us Mod Browser
				</h1>
				<h2 class="text-3xl mt-3 text-center">
					Easily find Among Us mods. Compatible with Android & PC.
				</h2>

				<div class="flex flex-wrap items-center gap-5 mt-5 justify-center">
					<UButton
						label="Explore Mods"
						icon="proicons:cube"
						to="/mods"
						class="px-8 py-4 text-2xl text-white bg-primary/40 backdrop-blur-md rounded-lg ring-1 ring-primary/20 hover:bg-primary/30 transform hover:scale-105 transition"
					/>
					<UButton
						label="Join Our Discord"
						color="neutral"
						variant="outline"
						icon="ic:baseline-discord"
						to="https://discord.gg/XdATcGSQBa"
						target="_blank"
						class="px-8 py-4 text-2xl text-white bg-white/10 backdrop-blur-md rounded-lg ring-1 ring-white/20 hover:bg-white/20 transform hover:scale-105 transition"
					/>
				</div>
			</div>
			<div class="mt-15" />
			<HomeModCarousel :speed="2" />
			<HomeModCarousel :speed="-2" />
		</div>
	</div>
</template>

<script setup lang="ts">
	import HomeModCarousel from "~/components/homepage/HomeModCarousel.vue";
</script>
