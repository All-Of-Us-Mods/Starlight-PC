<template>
	<div class="relative min-h-screen">
		<div class="absolute inset-0 bg-cover bg-center" style="background-image: url('/img/bg.jpg');" />
		<div class="absolute inset-0 bg-black/50" />

		<div class="page-content">
			<div class="relative flex flex-col items-center gap-5 text-white px-4 pt-20 md:pt-40">
				<h1 class="font-bold text-7xl text-white text-center">
					Starlight Launcher
				</h1>
				<h2 class="text-3xl mt-3 text-center">
					Launch Among Us mods with ease.
				</h2>

				<div class="flex flex-wrap items-center gap-5 mt-5 justify-center">
					<UTooltip text="Currently unavailable. Coming soon!">
						<UButton
							label="Install for Android"
							disabled
							icon="material-symbols:android"
							to="/"
							target="_blank"
							class="px-8 py-4 text-2xl"
						/>
					</UTooltip>
				</div>
			</div>
			<div class="mt-15" />
		</div>
	</div>
</template>

<script setup lang="ts">
</script>
