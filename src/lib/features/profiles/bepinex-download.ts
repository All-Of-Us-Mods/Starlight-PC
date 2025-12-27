import JSZip from 'jszip';
import { mkdir, writeFile } from '@tauri-apps/plugin-fs';
import { join, dirname } from '@tauri-apps/api/path';
import { fetch } from '@tauri-apps/plugin-http';

export async function downloadBepInEx(profilePath: string, bepinexUrl: string): Promise<void> {
	// This fetch runs through Rust, bypassing browser CORS
	const response = await fetch(bepinexUrl, {
		method: 'GET',
		connectTimeout: 30000
	});

	if (!response.ok) {
		throw new Error(`Failed to download BepInEx: ${response.statusText}`);
	}

	const arrayBuffer = await response.arrayBuffer();
	const zip = await JSZip.loadAsync(arrayBuffer);

	for (const [filename, file] of Object.entries(zip.files)) {
		const filePath = await join(profilePath, filename);

		if (file.dir) {
			await mkdir(filePath, { recursive: true });
		} else {
			// Safety: Ensure the directory for this file exists
			const parentDir = await dirname(filePath);
			await mkdir(parentDir, { recursive: true });

			const content = await file.async('uint8array');
			await writeFile(filePath, content);
		}
	}
}
