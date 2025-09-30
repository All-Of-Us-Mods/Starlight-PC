<template>
	<header class="fixed top-0 left-0 w-full z-50 bg-transparent backdrop-blur-sm px-4 py-5">
		<div class="max-w-7xl mx-auto flex items-center justify-between h-16 px-4 relative z-10">
			<!-- Left Logo -->
			<NuxtLink to="/" class="flex items-center">
				<h1 class="text-3xl font-bold flex items-center gap-2 hover:text-primary transition duration-300 ease-in-out hover:rotate-2 group">
					<UIcon
						name="material-symbols:star-shine"
						class="size-10 text-primary transform transition duration-300 group-hover:scale-125 group-hover:rotate-360 group-hover:text-yellow-400"
					/>
					Starlight
				</h1>

				<!-- Header Buttons -->
				<div class="ml-5">
					<UButton
						label="Explore Mods"
						icon="proicons:cube"
						to="/mods"
						class="header-btn"
					/>
					<UButton
						label="Install Launcher"
						icon="material-symbols:download"
						to="/installs"
						class="header-btn"
					/>
					<UButton
						label="Join Our Discord"
						icon="ic:baseline-discord"
						to="https://discord.gg/XdATcGSQBa"
						target="_blank"
						class="header-btn"
					/>
				</div>
			</NuxtLink>

			<!-- Right Sign In -->
			<div class="flex items-center">
				<UButton
					label="Sign In"
					icon="lets-icons:sign-in-squre"
					target="_blank"
					:to="`${$config.public.apiUrl}/login`"
					color="primary"
					class="px-4 py-1 text-xl transform hover:scale-103 transition"
				/>
			</div>
		</div>
	</header>
</template>

<script setup lang="ts">
	const { $config } = useNuxtApp();
</script>
