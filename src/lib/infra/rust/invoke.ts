import { invoke } from '@tauri-apps/api/core';
import { error as logError } from '@tauri-apps/plugin-log';
import type { RustCommandArgs, RustCommandArgsInput, RustCommandName, RustCommandResult } from './commands';

export class AppInvokeError extends Error {
	command: RustCommandName;
	cause?: unknown;

	constructor(command: RustCommandName, message: string, cause?: unknown) {
		super(message);
		this.name = 'AppInvokeError';
		this.command = command;
		this.cause = cause;
	}
}

export async function rustInvoke<T extends RustCommandName>(
	command: T,
	args?: RustCommandArgsInput<T>
): Promise<RustCommandResult<T>> {
	try {
		if (args === undefined) {
			return await invoke<RustCommandResult<T>>(command);
		}
		return await invoke<RustCommandResult<T>>(command, { args: args as RustCommandArgs<T> });
	} catch (cause) {
		const message = cause instanceof Error ? cause.message : String(cause);
		const wrapped = new AppInvokeError(command, message, cause);
		void logError(`[rustInvoke] ${command} failed: ${message}`);
		throw wrapped;
	}
}
