<template>
	<div class="relative min-h-screen">
		<!-- Background -->
		<div class="absolute inset-0 bg-cover bg-center" style="background-image: url('/img/bg.jpg');" />
		<div class="absolute inset-0 bg-black/50" />

		<div class="page-content relative flex flex-col gap-5 text-white px-4 pt-20 md:pt-30">
			<div class="flex flex-col md:flex-row gap-6 w-full max-w-6xl mx-auto">
				<!-- Left -->
				<div class="w-full md:w-64 flex flex-col gap-4">
					<div class="bg-white/10 p-4 rounded-lg text-white flex flex-col gap-2">
						<h3 class="text-lg font-semibold mb-2">
							Supported Platforms
						</h3>
						<UCheckboxGroup
							v-model="platformDefault"
							style="
                  --ui-border-accented: gray;
                  --ui-border-muted: gray;
                "
							variant="table"
							class="cursor-pointer [&_label]:cursor-pointer [&_input]:cursor-pointer"
							:items="platforms"
						/>
					</div>

					<div class="bg-white/10 p-4 rounded-lg text-white flex flex-col gap-2">
						<h3 class="text-lg font-semibold mb-2">
							Mod Type
						</h3>
						<URadioGroup
							v-model="defaultModType"
							style="
                  --ui-border-accented: gray;
                  --ui-border-muted: gray;
                  --ui-text-muted: #F5F5F5;"
							variant="table"
							class="cursor-pointer [&_label]:cursor-pointer [&_input]:cursor-pointer"
							:items="modTypes"
						/>
					</div>
				</div>

				<!-- Right -->
				<ul class="flex-1 space-y-3">
					<ModSearchList />
				</ul>
			</div>
		</div>
	</div>
</template>

<script setup lang="ts">
	import type { CheckboxGroupItem, CheckboxGroupValue } from "@nuxt/ui";
	import ModSearchList from "~/components/modspage/ModSearchList.vue";

	const platforms = ref<CheckboxGroupItem[]>(["PC", "Android"]);
	const platformDefault = ref<CheckboxGroupValue[]>(["PC", "Android"]);
	const modTypes = ref<CheckboxGroupItem[]>([
		{
			label: "Client Required",
			description: "Everyone must have the mods installed to play.",
			value: "client"
		},
		{
			label: "Host Only",
			description: "Only the host needs the mods installed.",
			value: "host"
		},
		{
			label: "Client Sided",
			description: "Mods that only change your experience, and not others.",
			value: "local"
		}
	]);
	const defaultModType = ref<string>("client");
</script>
