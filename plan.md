# Implementation Plan: Mod Profiles

This document outlines the architecture and phases for implementing a profile-based mod management system using Tauri, Svelte, and Rust.

## Architecture Overview

**Frontend-First Approach**: Leverage Tauri plugins (`@tauri-apps/plugin-store`, `@tauri-apps/plugin-fs`, `@tauri-apps/plugin-shell`) for 90% of operations. Only 2 custom Rust commands are required for low-level OS interactions.

---

## Phase 1: Dependencies & Configuration

### 1.1 Install Dependencies

**Frontend** (`package.json`):

```bash
bun add @tauri-apps/plugin-shell jszip
```

**Backend** (`src-tauri/Cargo.toml`):

```toml
[dependencies]
tauri-plugin-shell = "2"
sysinfo = "0.30" # Ensure sysinfo is present
```

### 1.2 Register Shell Plugin

**File**: `src-tauri/src/lib.rs`

```rust
.plugin(tauri_plugin_shell::init())
```

### 1.3 Update Capabilities

**File**: `src-tauri/capabilities/default.json`

```json
{
	"permissions": [
		"shell:allow-execute",
		"shell:allow-spawn",
		{
			"identifier": "shell:allow-execute",
			"allow": [
				{
					"name": "launch-among-us",
					"cmd": "Among Us.exe",
					"args": true
				}
			]
		}
	]
}
```

---

## Phase 2: Backend Commands

### 2.1 Create Profile Backend Commands

**File**: `src-tauri/src/commands/profiles_backend.rs`

```rust
use std::path::Path;
use tauri::Manager;

/// Windows-only: Set DLL directory for Doorstop.
/// Required because Tauri Shell plugin doesn't expose this environment tweak.
#[tauri::command]
#[cfg(windows)]
pub fn set_dll_directory(profile_path: String) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;

    let wide: Vec<u16> = Path::new(&profile_path)
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let success = unsafe { SetDllDirectoryW(wide.as_ptr()) };

    if success == 0 {
        Err(format!(
            "Failed to set DLL directory: {}",
            std::io::Error::last_os_error()
        ))
    } else {
        Ok(())
    }
}

#[cfg(windows)]
#[link(name = "Kernel32")]
extern "system" {
    fn SetDllDirectoryW(lp_path_name: *const u16) -> i32;
}

/// Check if Among Us is already running.
#[tauri::command]
pub fn check_among_us_running() -> Result<bool, String> {
    use sysinfo::{ProcessExt, System, SystemExt};

    let mut sys = System::new_all();
    sys.refresh_all();

    Ok(sys.processes_by_exact_name("Among Us").next().is_some())
}

#[cfg(not(windows))]
#[tauri::command]
pub fn set_dll_directory(_profile_path: String) -> Result<(), String> {
    Ok(())
}
```

### 2.2 Register Commands

**File**: `src-tauri/src/lib.rs`

```rust
mod profiles_backend;

.invoke_handler(tauri::generate_handler![
    commands::profiles_backend::set_dll_directory,
    commands::profiles_backend::check_among_us_running
])
```

---

## Phase 3: Frontend Schema & Types

### 3.1 Profile Schema

**File**: `src/lib/features/profiles/schema.ts`

```ts
import { type } from 'arktype';

export const ProfileEntry = type({
	id: 'string',
	name: 'string <= 100',
	path: 'string',
	created_at: 'number',
	'last_launched_at?': 'number',
	mod_ids: 'string[]'
});

export type Profile = typeof ProfileEntry.infer;
```

### 3.2 Settings Schema

**File**: `src/lib/features/settings/schema.ts`

```ts
import { type } from 'arktype';

export const Settings = type({
	bepinex_url: 'string',
	bepinex_version: 'string',
	among_us_path: 'string',
	close_on_launch: 'boolean',
	'last_launched_profile_id?': 'string'
});

export type AppSettings = typeof Settings.infer;
```

---

## Phase 4: Profile Service (Core CRUD)

**File**: `src/lib/features/profiles/profile-service.svelte.ts`

```ts
import { Store } from '@tauri-apps/plugin-store';
import { mkdir, remove } from '@tauri-apps/plugin-fs';
import { join } from '@tauri-apps/api/path';
import { ProfileEntry, type Profile } from './schema';
import { downloadBepInEx } from './bepinex-download';

class ProfileService {
	async getStore(): Promise<Store> {
		return await Store.load('registry.json');
	}

	async getProfiles(): Promise<Profile[]> {
		const store = await this.getStore();
		const profiles = (await store.get<Profile[]>('profiles')) ?? [];

		return profiles.sort((a, b) => {
			const aLaunched = a.last_launched_at ?? 0;
			const bLaunched = b.last_launched_at ?? 0;
			if (aLaunched !== bLaunched) return bLaunched - aLaunched;
			return b.created_at - a.created_at;
		});
	}

	async createProfile(name: string): Promise<Profile> {
		const trimmed = name.trim();
		if (!trimmed) throw new Error('Profile name cannot be empty');

		const store = await this.getStore();
		const profiles = await this.getProfiles();

		if (profiles.some((p) => p.name.toLowerCase() === trimmed.toLowerCase())) {
			throw new Error(`Profile '${trimmed}' already exists`);
		}

		const dataDir = await this.getAppDataDir();
		const profilesDir = await join(dataDir, 'profiles');
		const timestamp = Date.now();
		const slug = this.slugify(trimmed);
		const profileId = slug ? `${slug}-${timestamp}` : `profile-${timestamp}`;
		const profilePath = await join(profilesDir, profileId);

		await mkdir(profilePath, { recursive: true });

		const bepinexUrl = await this.getBepInExUrl();
		await downloadBepInEx(profilePath, bepinexUrl);

		const profile: Profile = {
			id: profileId,
			name: trimmed,
			path: profilePath,
			created_at: timestamp,
			last_launched_at: undefined,
			mod_ids: []
		};

		profiles.push(profile);
		await store.set('profiles', profiles);
		await store.save();
		return profile;
	}

	async deleteProfile(profileId: string): Promise<void> {
		const store = await this.getStore();
		const profiles = await this.getProfiles();
		const profile = profiles.find((p) => p.id === profileId);

		if (!profile) throw new Error(`Profile '${profileId}' not found`);

		await remove(profile.path, { recursive: true });
		await store.set(
			'profiles',
			profiles.filter((p) => p.id !== profileId)
		);
		await store.save();
	}

	async getActiveProfile(): Promise<Profile | null> {
		const store = await this.getStore();
		const lastId = await store.get<string>('last_launched_profile_id');
		if (!lastId) return null;

		const profiles = await this.getProfiles();
		return profiles.find((p) => p.id === lastId) ?? null;
	}

	async updateLastLaunched(profileId: string): Promise<void> {
		const store = await this.getStore();
		const profiles = await this.getProfiles();
		const profile = profiles.find((p) => p.id === profileId);

		if (profile) {
			profile.last_launched_at = Date.now();
			await store.set('profiles', profiles);
			await store.set('last_launched_profile_id', profileId);
			await store.save();
		}
	}

	private slugify(input: string): string {
		return input
			.toLowerCase()
			.replace(/[^a-z0-9]/g, '-')
			.replace(/-+/g, '-')
			.replace(/^-|-$/g, '');
	}

	private async getAppDataDir(): Promise<string> {
		const { appDataDir } = await import('@tauri-apps/api/path');
		return await appDataDir();
	}

	private async getBepInExUrl(): Promise<string> {
		const store = await this.getStore();
		const settings = await store.get<{ bepinex_url: string }>('settings');
		return settings?.bepinex_url ?? DEFAULT_BEPINEX_URL;
	}
}

const DEFAULT_BEPINEX_URL =
	'https://builds.bepinex.dev/projects/bepinex_be/738/BepInEx-Unity.IL2CPP-win-x86-6.0.0-be.738%2Baf0cba7.zip';
export const profileService = new ProfileService();
```

---

## Phase 5: Launch Service

**File**: `src/lib/features/profiles/launch-service.svelte.ts`

```ts
import { invoke } from '@tauri-apps/api/core';
import { Command } from '@tauri-apps/plugin-shell';
import { Store } from '@tauri-apps/plugin-store';
import { profileService } from './profile-service';
import type { Profile } from './schema';

class LaunchService {
	async launchProfile(profile: Profile): Promise<void> {
		const store = await Store.load('registry.json');
		const settings = (await store.get<any>('settings')) ?? {};

		if (!settings.among_us_path) {
			throw new Error('Among Us path not configured.');
		}

		const isRunning = await invoke<boolean>('check_among_us_running');
		if (isRunning) throw new Error('Among Us is already running');

		await invoke('set_dll_directory', { profilePath: profile.path });

		const args = [
			'--doorstop-enabled',
			'true',
			'--doorstop-target-assembly',
			`${profile.path}/BepInEx/core/BepInEx.Unity.IL2CPP.dll`,
			'--doorstop-clr-corlib-dir',
			`${profile.path}/dotnet`,
			'--doorstop-clr-runtime-coreclr-path',
			`${profile.path}/dotnet/coreclr.dll`
		];

		const command = Command.create('launch-among-us', args, {
			cwd: settings.among_us_path
		});

		await command.spawn();
		await profileService.updateLastLaunched(profile.id);

		if (settings.close_on_launch) {
			const { getCurrentWindow } = await import('@tauri-apps/api/window');
			getCurrentWindow().close();
		}
	}

	async launchVanilla(): Promise<void> {
		const store = await Store.load('registry.json');
		const settings = (await store.get<any>('settings')) ?? {};

		const command = Command.create('launch-among-us', [], {
			cwd: settings.among_us_path
		});

		await command.spawn();
		await store.set('last_launched_profile_id', null);
		await store.save();
	}
}

export const launchService = new LaunchService();
```

---

## Phase 6: BepInEx Download Utility

**File**: `src/lib/features/profiles/bepinex-download.ts`

```ts
import JSZip from 'jszip';
import { mkdir, writeFile } from '@tauri-apps/plugin-fs';
import { join } from '@tauri-apps/api/path';

export async function downloadBepInEx(profilePath: string, bepinexUrl: string): Promise<void> {
	const response = await fetch(bepinexUrl);
	if (!response.ok) throw new Error('Failed to download BepInEx');

	const arrayBuffer = await response.arrayBuffer();
	const zip = await JSZip.loadAsync(arrayBuffer);

	for (const [filename, file] of Object.entries(zip.files)) {
		const filePath = await join(profilePath, filename);
		if (file.dir) {
			await mkdir(filePath, { recursive: true });
		} else {
			const content = await file.async('uint8array');
			await writeFile(filePath, content);
		}
	}
}
```

---

## Phase 7: Mod Installation Service

**File**: `src/lib/features/profiles/mod-install-service.svelte.ts`

```ts
import { writeFile, mkdir, remove } from '@tauri-apps/plugin-fs';
import { join } from '@tauri-apps/api/path';

class ModInstallService {
	async installModToProfile(modId: string, profilePath: string): Promise<void> {
		const response = await fetch(`https://api.example.com/mods/${modId}/download`);
		if (!response.ok) throw new Error('Download failed');

		const data = new Uint8Array(await response.arrayBuffer());
		const pluginsDir = await join(profilePath, 'BepInEx', 'plugins');

		await mkdir(pluginsDir, { recursive: true });
		await writeFile(await join(pluginsDir, `${modId}.dll`), data);
	}

	async removeModFromProfile(modId: string, profilePath: string): Promise<void> {
		const dllPath = await join(profilePath, 'BepInEx', 'plugins', `${modId}.dll`);
		await remove(dllPath);
	}
}

export const modInstallService = new ModInstallService();
```

---

## Phase 8: Settings Service

**File**: `src/lib/features/settings/settings-service.svelte.ts`

```ts
import { Store } from '@tauri-apps/plugin-store';
import type { AppSettings } from './schema';

class SettingsService {
	async getSettings(): Promise<AppSettings> {
		const store = await Store.load('registry.json');
		return (
			(await store.get<AppSettings>('settings')) ?? {
				bepinex_url: 'https://...',
				bepinex_version: '6.0.0-be.738',
				among_us_path: '',
				close_on_launch: false
			}
		);
	}

	async updateSettings(updates: Partial<AppSettings>): Promise<void> {
		const store = await Store.load('registry.json');
		const current = await this.getSettings();
		await store.set('settings', { ...current, ...updates });
		await store.save();
	}
}

export const settingsService = new SettingsService();
```

---

## Phase 9: TanStack Query Integration

**File**: `src/lib/features/profiles/queries.ts`

```ts
import { queryOptions } from '@tanstack/svelte-query';
import { profileService } from './profile-service';
import { settingsService } from '../settings/settings-service';

export const profileQueries = {
	all: () =>
		queryOptions({
			queryKey: ['profiles'],
			queryFn: () => profileService.getProfiles()
		}),
	active: () =>
		queryOptions({
			queryKey: ['profiles', 'active'],
			queryFn: () => profileService.getActiveProfile()
		})
};

export const settingsQueries = {
	get: () =>
		queryOptions({
			queryKey: ['settings'],
			queryFn: () => settingsService.getSettings()
		})
};
```

---

## Phase 10: UI Components

### 10.1 Profile Card

**File**: `src/lib/features/profiles/components/ProfileCard.svelte`

- Displays name, mod count, and last launch date.
- Provides "Launch", "Open Folder", and "Delete" actions.

### 10.2 Create Profile Dialog

**File**: `src/lib/features/profiles/components/CreateProfileDialog.svelte`

- Input for profile name.
- Shows download progress during BepInEx setup.

### 10.3 Add to Profile Dialog

**File**: `src/lib/features/profiles/components/AddToProfileDialog.svelte`

- Triggered from the Mod Browser.
- Dropdown to select a profile for installation.

---

## Phase 11: Route Implementation

- **Settings Page** (`/settings`): Configure Among Us path and BepInEx defaults.
- **Library Page** (`/library`): Manage and launch profiles or Vanilla game.
- **AppShell**: Add "Launch Last Used" quick-action to the top navigation bar.

---

## Implementation Checklist

1. [ ] **Dependencies**: Install Bun and Cargo packages.
2. [ ] **Backend**: Implement Rust DLL directory and process check commands.
3. [ ] **Storage**: Define ArkType schemas for validation.
4. [ ] **Logic**: Build Profile and Launch services.
5. [ ] **FS**: Implement Zip extraction and Mod DLL writing.
6. [ ] **UI**: Build Svelte components and integrate TanStack Query.
7. [ ] **Test**: Verify launch arguments and directory isolation.

## Summary

This plan ensures a clean separation of concerns, utilizing Tauri's robust plugin ecosystem to minimize complex Rust code while providing a modern, reactive frontend for mod management.
