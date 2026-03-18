import type { MutationOptions } from '@tanstack/svelte-query';
import type { RustCommandArgsInput, RustCommandName, RustCommandResult } from './commands';
import { type AppInvokeError, rustInvoke } from './invoke';

type RustMutationConfig<TCommand extends RustCommandName> = Omit<
	MutationOptions<RustCommandResult<TCommand>, AppInvokeError, RustCommandArgsInput<TCommand>>,
	'mutationFn'
> & {
	command: TCommand;
};

export function rustMutationOptions<TCommand extends RustCommandName>(
	config: RustMutationConfig<TCommand>
) {
	const { command, ...options } = config;
	return {
		...options,
		mutationFn: (args: RustCommandArgsInput<TCommand>) => rustInvoke(command, args)
	};
}
