import { queryOptions, type QueryKey } from '@tanstack/svelte-query';
import type { RustCommandArgsInput, RustCommandName, RustCommandResult } from './commands';
import { rustInvoke } from './invoke';

type BaseQueryConfig<TCommand extends RustCommandName, TQueryKey extends QueryKey> = Omit<
	Parameters<
		typeof queryOptions<RustCommandResult<TCommand>, Error, RustCommandResult<TCommand>, TQueryKey>
	>[0],
	'queryFn'
>;

type RustQueryConfig<
	TCommand extends RustCommandName,
	TQueryKey extends QueryKey
> = BaseQueryConfig<TCommand, TQueryKey> & {
	command: TCommand;
	args?: RustCommandArgsInput<TCommand>;
};

export function rustQueryOptions<TCommand extends RustCommandName, TQueryKey extends QueryKey>(
	config: RustQueryConfig<TCommand, TQueryKey>
) {
	const { command, args, ...options } = config;
	return queryOptions({
		...options,
		queryFn: () => rustInvoke(command, args),
		networkMode: 'always'
	});
}
