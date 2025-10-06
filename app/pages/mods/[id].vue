<template>
	<div class="relative min-h-screen">
		<div class="absolute inset-0 bg-cover bg-center" style="background-image: url('/img/bg.jpg');" />
		<div class="absolute inset-0 bg-black/50" />

		<div class="page-content relative flex flex-col gap-5 text-white px-4 pt-20 md:pt-30">
			<div class="flex flex-col justify-center md:flex-row gap-6 w-full max-w-6xl mx-auto">
				<div v-if="pending" class="flex p-8">
					<UIcon name="mingcute:loading-fill" class="size-10 animate-spin" />
				</div>

				<div v-else-if="error" class="p-4 flex flex-col items-center gap-2">
					<span class="font-semibold text-2xl text-white">
						Could not find mod with ID: {{ modId }}
					</span>
					<span class="text-base text-white/70">{{ error }}</span>
					<UButton
						label="Back to Mods"
						icon="proicons:cube"
						to="/mods"
						class="px-4 py-2 text-xl mt-3 text-white bg-primary/40 backdrop-blur-md rounded-lg ring-1 ring-primary/20 hover:bg-primary/30 transform hover:scale-105 transition"
					/>
				</div>

				<div v-else class="flex flex-col md:flex-row gap-8 w-full">
					<div class="flex-shrink-0">
						<img
							:src="`${config.public.apiUrl}${mod.thumbnail}`"
							:alt="mod.mod_name"
							class="w-full md:w-64 h-64 rounded-xl object-contain shadow-2xl bg-black/30 backdrop-blur-sm p-4"
						>
					</div>

					<div class="flex-1 flex flex-col gap-6">
						<div>
							<h1 class="text-4xl font-bold text-white mb-2">
								{{ mod.mod_name }}
							</h1>
							<p class="text-xl text-white/70">
								by {{ mod.author }}
							</p>
						</div>

						<div class="flex flex-wrap gap-4 text-sm">
							<div class="flex items-center gap-2 text-white/70">
								<UIcon name="material-symbols:download" class="size-5" />
								<span>{{ mod.downloads ?? 0 }} downloads</span>
							</div>
							<div class="flex items-center gap-2 text-white/70">
								<UIcon name="material-symbols:calendar-today" class="size-5" />
								<span>Created {{ formatTimeAgo(mod.created_at) }}</span>
							</div>
							<div v-if="mod.updated_at" class="flex items-center gap-2 text-white/70">
								<UIcon name="material-symbols:sync-outline" class="size-5" />
								<span>Updated {{ formatTimeAgo(mod.updated_at) }}</span>
							</div>
						</div>

						<div>
							<h2 class="text-2xl font-semibold text-white mb-2">
								Description
							</h2>
							<p class="text-white/80 text-lg leading-relaxed">
								{{ mod.description || "No description available." }}
							</p>
						</div>

						<div>
							<h2 class="text-2xl font-semibold text-white mb-3">
								Available Versions
							</h2>

							<div
								v-if="modsFolderLoading"
								class="mb-3 p-3 bg-blue-500/20 border border-blue-500/40 rounded-lg text-white/90"
							>
								Detecting your Among Us installation...
							</div>
							<div
								v-else-if="modsFolderError"
								class="mb-3 p-3 bg-red-500/20 border border-red-500/40 rounded-lg text-white"
							>
								{{ modsFolderError }}
							</div>
							<div
								v-else-if="modsFolder"
								class="mb-3 p-3 bg-emerald-500/10 border border-emerald-500/30 rounded-lg text-white/80"
							>
								Mods will be installed to <span class="font-semibold">{{ modsFolder }}</span>.
							</div>

							<div v-if="versionsLoading" class="flex justify-center p-4">
								<UIcon name="mingcute:loading-fill" class="size-6 animate-spin" />
							</div>

							<div v-else-if="versionsError" class="p-4 bg-red-500/20 border border-red-500/50 rounded-lg">
								<p class="text-white">
									Failed to load versions
								</p>
							</div>

							<div v-else-if="sortedVersions.length > 0" class="space-y-2">
								<div
									v-for="version in sortedVersions"
									:key="version.self || version.version"
									class="flex items-center justify-between p-4 bg-white/5 hover:bg-white/10 rounded-lg border border-white/10 transition-colors"
								>
									<div class="flex-1">
										<div class="flex items-center gap-3">
											<span class="text-lg font-bold text-white">v{{ version.version }}</span>
											<span class="px-2 py-1 text-xs font-medium rounded-full bg-primary/30 text-primary-200 border border-primary/50">
												{{ version.platform }}
											</span>
										</div>
										<div class="flex flex-wrap items-center gap-x-6 gap-y-2 mt-2 text-sm text-white/60">
											<div class="flex items-center gap-1">
												<UIcon name="material-symbols:download" class="size-4" />
												<span>{{ version.downloads ?? 0 }} downloads</span>
											</div>
											<div class="flex items-center gap-1">
												<UIcon name="material-symbols:calendar-today" class="size-4" />
												<span>{{ formatTimeAgo(version.created_at) }}</span>
											</div>
											<div class="flex items-center gap-1">
												<UIcon name="mdi:file-outline" class="size-4" />
												<span>{{ version.file_name || `${mod.mod_id || "mod"}_${version.version}.dll` }}</span>
											</div>
											<div v-if="version.checksum" class="flex items-center gap-1">
												<UIcon name="mdi:fingerprint" class="size-4" />
												<span class="font-mono text-xs text-white/70 break-all">{{ version.checksum }}</span>
											</div>
										</div>
									</div>
									<UButton
										label="Install"
										icon="material-symbols:download"
										size="sm"
										:loading="isDownloading && selectedVersion === version.version"
										:disabled="isDownloading || !modsFolder || modsFolderLoading"
										class="bg-primary/40 hover:bg-primary/50"
										@click="downloadModVersion(version)"
									/>
								</div>
							</div>

							<div v-else class="p-4 bg-white/5 rounded-lg border border-white/10">
								<p class="text-white/60 text-center">
									No versions available
								</p>
							</div>
						</div>

						<div class="flex gap-4">
							<UButton
								label="Install Mod"
								icon="material-symbols:download"
								size="lg"
								:loading="isDownloading"
								:disabled="isDownloading || !sortedVersions.length || !modsFolder || modsFolderLoading"
								class="px-6 py-3 text-white bg-primary/40 backdrop-blur-md rounded-lg ring-1 ring-primary/20 hover:bg-primary/30 transform hover:scale-105 transition"
								@click="downloadMod"
							/>
							<UButton
								label="Back to Mods"
								icon="material-symbols:arrow-back"
								to="/mods"
								size="lg"
								variant="outline"
								class="px-6 py-3"
							/>
						</div>

						<div
							v-if="downloadStatus"
							class="mt-2 p-4 rounded-lg"
							:class="downloadStatus.type === 'success'
								? 'bg-green-500/20 border border-green-500/50'
								: downloadStatus.type === 'error'
									? 'bg-red-500/20 border border-red-500/50'
									: 'bg-blue-500/20 border border-blue-500/50'"
						>
							<div class="flex items-center gap-2">
								<UIcon
									:name="downloadStatus.type === 'success'
										? 'material-symbols:check-circle'
										: downloadStatus.type === 'error'
											? 'material-symbols:error'
											: 'material-symbols:info'"
									class="size-5"
								/>
								<span class="text-white font-medium">{{ downloadStatus.message }}</span>
							</div>
							<p v-if="downloadStatus.details" class="text-sm text-white/70 mt-2">
								{{ downloadStatus.details }}
							</p>
						</div>
					</div>
				</div>
			</div>
		</div>
	</div>
</template>

<script setup lang="ts">
	import type { Mod, ModVersion } from "~/types";
	import { invoke } from "@tauri-apps/api/core";

	interface DownloadStatus {
		type: "info" | "success" | "error"
		message: string
		details?: string
	}

	const route = useRoute();
	const modId = String((route.params as { id?: string }).id || "");

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

	const { data: mod, pending, error } = await useLazyFetch<Mod>(`/api/v1/mods/${modId}`, {
		baseURL: config.public.apiUrl,
		default: () => ({} as Mod),
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

	const versions = ref<ModVersion[]>([]);
	const versionsLoading = ref(true);
	const versionsError = ref<string | null>(null);

	const modsFolder = ref<string | null>(null);
	const modsFolderError = ref<string | null>(null);
	const modsFolderLoading = ref(false);

	const sanitizePathSegment = (segment: string) => segment.replace(/^[\\/]+|[\\/]+$/g, "");

	const joinPath = (basePath: string, ...segments: string[]) => {
		const separator = basePath.includes("\\") ? "\\" : "/";
		const normalizedBase = basePath.replace(/[\\/]+$/, "");
		const cleanedSegments = segments.map((segment) => sanitizePathSegment(segment));

		return [normalizedBase, ...cleanedSegments].join(separator);
	};

	const resolveModsFolderPath = (installationPath: string) => {
		return joinPath(installationPath, "BepInEx", "plugins");
	};

	const loadModsFolder = async () => {
		modsFolderLoading.value = true;
		modsFolderError.value = null;

		try {
			const paths = await invoke<string[]>("get_among_us_paths");
			const firstValid = (paths ?? []).find((value) => typeof value === "string" && value.trim().length > 0);

			if (firstValid) {
				modsFolder.value = resolveModsFolderPath(firstValid.trim());
			} else {
				modsFolder.value = null;
				modsFolderError.value = "Among Us installation not found.";
			}
		} catch (err) {
			modsFolder.value = null;
			modsFolderError.value = err instanceof Error ? err.message : "Failed to detect Among Us installation.";
		} finally {
			modsFolderLoading.value = false;
		}
	};

	const normalizeTimestamp = (value: ModVersion["created_at"]) => {
		if (value === undefined || value === null) {
			return 0;
		}

		if (typeof value === "number") {
			return value;
		}

		const trimmed = String(value).trim();
		const numeric = Number(trimmed);

		if (Number.isFinite(numeric)) {
			return numeric;
		}

		const parsed = new Date(trimmed).getTime();

		return Number.isNaN(parsed) ? 0 : parsed;
	};

	const fetchVersions = async () => {
		versionsLoading.value = true;
		versionsError.value = null;

		try {
			const baseVersions = await $fetch<ModVersion[]>(
				`/api/v1/mods/${modId}/versions`,
				{
					baseURL: config.public.apiUrl,
					retry: 3,
					retryDelay: 1000,
					timeout: 10000
				}
			);

			const detailedVersions = await Promise.all(
				(baseVersions ?? []).map(async (version) => {
					if (!version?.version) {
						return version;
					}

					try {
						const detail = await $fetch<ModVersion>(
							`/api/v1/mods/${modId}/versions/${version.version}`,
							{
								baseURL: config.public.apiUrl,
								retry: 2,
								retryDelay: 500,
								timeout: 10000
							}
						);

						return {
							...version,
							...detail
						};
					} catch (detailError) {
						console.error("Failed to fetch version detail", version.version, detailError);
						return version;
					}
				})
			);

			versions.value = detailedVersions ?? [];
		} catch (err) {
			console.error("Could not load versions", err);
			versionsError.value = err instanceof Error ? err.message : "Failed to load versions";
		} finally {
			versionsLoading.value = false;
		}
	};

	onMounted(() => {
		// Only fetch on client to avoid SSR issues with window/crypto APIs
		fetchVersions();
		loadModsFolder();
	});

	const sortedVersions = computed<ModVersion[]>(() => {
		return [...(versions.value ?? [])].sort(
			(a, b) => normalizeTimestamp(b?.created_at) - normalizeTimestamp(a?.created_at)
		);
	});

	const isDownloading = ref(false);
	const selectedVersion = ref<string | null>(null);
	const downloadStatus = ref<DownloadStatus | null>(null);

	const computeChecksum = async (buffer: ArrayBuffer) => {
		const hash = await crypto.subtle.digest("SHA-256", buffer);
		return Array.from(new Uint8Array(hash))
			.map((byte) => byte.toString(16).padStart(2, "0"))
			.join("");
	};

	const downloadModVersion = async (version: ModVersion) => {
		if (!mod.value || !version.version) return;

		if (!modsFolder.value) {
			downloadStatus.value = {
				type: "error",
				message: "Among Us installation not found",
				details: modsFolderError.value ?? "Please launch the Among Us desktop app at least once."
			};
			return;
		}

		const targetDirectory = modsFolder.value as string;

		selectedVersion.value = version.version;
		isDownloading.value = true;
		downloadStatus.value = {
			type: "info",
			message: "Starting download..."
		};

		try {
			// Use the actual version from the API
			const downloadUrl = `${config.public.apiUrl}/api/v1/mods/${modId}/versions/${version.version}/file`;
			const trimmed = version.file_name?.trim();
			const fileName = trimmed && trimmed.length > 0
				? trimmed
				: `${mod.value.mod_id || "mod"}_${version.version}.dll`;

			downloadStatus.value = {
				type: "info",
				message: `Downloading ${version.version}...`,
				details: `Platform: ${version.platform}${version.checksum ? ` | Checksum: ${version.checksum}` : ""}`
			};

			// Fetch the file
			const response = await fetch(downloadUrl);

			if (!response.ok) {
				throw new Error(`Failed to download: ${response.status} ${response.statusText}`);
			}

			downloadStatus.value = {
				type: "info",
				message: "Processing file..."
			};

			// Get the file as array buffer
			const arrayBuffer = await response.arrayBuffer();

			if (version.checksum) {
				downloadStatus.value = {
					type: "info",
					message: "Verifying checksum...",
					details: version.checksum
				};

				const actualChecksum = await computeChecksum(arrayBuffer);

				if (actualChecksum.toLowerCase() !== version.checksum.toLowerCase()) {
					throw new Error(`Checksum mismatch. Expected ${version.checksum}, got ${actualChecksum}`);
				}
			}

			const uint8Array = new Uint8Array(arrayBuffer);

			// Write to filesystem using Tauri
			const { writeFile, mkdir, exists } = await import("@tauri-apps/plugin-fs");

			// Ensure mods directory exists
			const dirExists = await exists(targetDirectory);

			if (!dirExists) {
				await mkdir(targetDirectory, { recursive: true });
			}

			const filePath = joinPath(targetDirectory, fileName);

			downloadStatus.value = {
				type: "info",
				message: "Saving file...",
				details: `Location: ${filePath}`
			};

			await writeFile(filePath, uint8Array);

			downloadStatus.value = {
				type: "success",
				message: `Version ${version.version} installed successfully!`,
				details: `Saved to: ${filePath}`
			};
		} catch (err) {
			console.error("Download error:", err);
			downloadStatus.value = {
				type: "error",
				message: "Failed to install mod",
				details: String(err)
			};
		} finally {
			isDownloading.value = false;
			selectedVersion.value = null;
		}
	};

	const downloadMod = async () => {
		const firstVersion = sortedVersions.value[0];

		if (firstVersion) {
			await downloadModVersion(firstVersion);
		} else {
			downloadStatus.value = {
				type: "error",
				message: "No versions available",
				details: "Please try again later"
			};
		}
	};
</script>
