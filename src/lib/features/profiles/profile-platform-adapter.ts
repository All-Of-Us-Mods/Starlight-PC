import { invoke } from '@tauri-apps/api/core';

class ProfilePlatformAdapter {
	readBinaryFile(path: string) {
		return invoke<Uint8Array>('profiles_read_binary_file', { args: { path } });
	}
}

export const profilePlatformAdapter = new ProfilePlatformAdapter();
