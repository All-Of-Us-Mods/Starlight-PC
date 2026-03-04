import { readFile } from '@tauri-apps/plugin-fs';

class ProfilePlatformAdapter {
	readBinaryFile(path: string) {
		return readFile(path);
	}
}

export const profilePlatformAdapter = new ProfilePlatformAdapter();
