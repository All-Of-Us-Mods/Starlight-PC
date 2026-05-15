<script lang="ts" module>
	import type { Profile } from '$lib/features/profiles/schema';

	export interface ProfileLogViewerProps {
		profile: Profile;
		isRunning: boolean;
	}
</script>

<script lang="ts">
	import * as Card from '$lib/components/ui/card';
	import { Button } from '$lib/components/ui/button';
	import { Badge } from '$lib/components/ui/badge';
	import { ScrollArea } from '$lib/components/ui/scroll-area';
	import * as Tabs from '$lib/components/ui/tabs';
	import { createQuery, useQueryClient } from '@tanstack/svelte-query';
	import { join } from '@tauri-apps/api/path';
	import { warn } from '@tauri-apps/plugin-log';
	import { onMount } from 'svelte';
	import { TerminalSquare, RefreshCw, Pause, Play, Eraser } from '@lucide/svelte';
	import { profileQueries } from '$lib/features/profiles/queries';
	import { profileLogKey } from '$lib/features/profiles/profile-keys';
	import { watchDirectory } from '$lib/infra/tauri/fs-watch';

	let { profile, isRunning }: ProfileLogViewerProps = $props();

	const LOG_FILES = ['LogOutput.log', 'ErrorLog.log'] as const;
	type LogFileName = (typeof LOG_FILES)[number];
	type LogLevel = 'error' | 'warn' | 'info' | 'debug' | 'trace' | 'unknown';
	type ParsedLogLine = { id: string; text: string; level: LogLevel };
	const MAX_LOG_LINES = 2000;

	const queryClient = useQueryClient();
	let activeLogFile = $state<LogFileName>('LogOutput.log');
	const logQuery = createQuery(() => profileQueries.log(profile.path, activeLogFile));

	let logPath = $state('');
	let logContainer = $state<HTMLDivElement | null>(null);
	let autoScrollEnabled = $state(true);
	let isViewCleared = $state(false);

	const parsedLines = $derived(parseProfileLog(logQuery.data ?? ''));
	const displayedLines = $derived(isViewCleared ? [] : parsedLines);

	function parseProfileLog(rawLog: string): ParsedLogLine[] {
		if (!rawLog.trim()) return [];
		const allLines = rawLog.split(/\r?\n/).filter((line) => line.length > 0);
		const start = Math.max(0, allLines.length - MAX_LOG_LINES);
		const visibleLines = allLines.slice(start);
		return visibleLines.map((text, index) => ({
			id: String(start + index),
			text,
			level: detectLogLevel(text)
		}));
	}

	function detectLogLevel(text: string): LogLevel {
		const normalized = text.toLowerCase();
		if (normalized.includes('fatal') || normalized.includes('error')) return 'error';
		if (normalized.includes('warn') || normalized.includes('warning')) return 'warn';
		if (normalized.includes('debug')) return 'debug';
		if (normalized.includes('trace')) return 'trace';
		if (normalized.includes('info')) return 'info';
		return 'unknown';
	}

	function levelToTextClass(level: LogLevel): string {
		switch (level) {
			case 'error':
				return 'text-red-400';
			case 'warn':
				return 'text-amber-300';
			case 'info':
				return 'text-yellow-300';
			case 'debug':
				return 'text-green-400';
			case 'trace':
				return 'text-cyan-300';
			default:
				return 'text-muted-foreground';
		}
	}

	$effect(() => {
		if (!autoScrollEnabled || !logContainer || displayedLines.length === 0) return;

		queueMicrotask(() => {
			if (!logContainer || !autoScrollEnabled) return;
			logContainer.scrollTop = logContainer.scrollHeight;
		});
	});

	async function refreshLog() {
		isViewCleared = false;
		await queryClient.invalidateQueries({ queryKey: profileLogKey(profile.path, activeLogFile) });
	}

	onMount(() => {
		let debounceTimer: ReturnType<typeof setTimeout> | undefined;
		let stopWatching: (() => void) | undefined;

		void (async () => {
			logPath = await join(profile.path, 'BepInEx', 'LogOutput.log');
			const logsDir = await join(profile.path, 'BepInEx');

			try {
				stopWatching = await watchDirectory(
					logsDir,
					() => {
						clearTimeout(debounceTimer);
						debounceTimer = setTimeout(() => {
							void Promise.all(
								LOG_FILES.map((fileName) =>
									queryClient.invalidateQueries({
										queryKey: profileLogKey(profile.path, fileName)
									})
								)
							);
						}, 200);
					},
					{ recursive: false }
				);
			} catch (error) {
				void warn(`Failed to watch profile logs directory: ${error}`);
			}
		})();

		return () => {
			clearTimeout(debounceTimer);
			stopWatching?.();
		};
	});

	$effect(() => {
		void (async () => {
			logPath = await join(profile.path, 'BepInEx', activeLogFile);
		})();
	});
</script>

<Card.Root class="mt-6 border-border/60 bg-card/50">
	<Card.Header class="pb-3">
		<Card.Title class="flex items-center gap-2 text-lg">
			<TerminalSquare class="size-5" />
			Profile Logs
		</Card.Title>
		<Card.Description class="space-y-1 text-xs">
			<p class="font-mono text-muted-foreground/90">{logPath || 'Loading log path...'}</p>
			<p>Switch between profile log files using tabs.</p>
		</Card.Description>
	</Card.Header>
	<Card.Content class="space-y-4">
		<div class="flex flex-wrap items-center gap-2">
			<Badge variant={isRunning ? 'default' : 'outline'}>{isRunning ? 'Running' : 'Idle'}</Badge>
			{#if parsedLines.length >= MAX_LOG_LINES}
				<Badge variant="outline">Showing last {MAX_LOG_LINES.toLocaleString()} lines</Badge>
			{/if}
		</div>

		<div class="flex flex-wrap items-center gap-2">
			<Button
				size="sm"
				variant="outline"
				class="gap-2"
				onclick={refreshLog}
				disabled={logQuery.isFetching}
			>
				<RefreshCw class={`size-4 ${logQuery.isFetching ? 'animate-spin' : ''}`} />
				Refresh
			</Button>
			<Button
				size="sm"
				variant="outline"
				class="gap-2"
				onclick={() => {
					isViewCleared = true;
				}}
				disabled={displayedLines.length === 0}
			>
				<Eraser class="size-4" />
				Clear view
			</Button>
			<Button
				size="sm"
				variant="outline"
				class="gap-2"
				onclick={() => {
					autoScrollEnabled = !autoScrollEnabled;
				}}
			>
				{#if autoScrollEnabled}
					<Pause class="size-4" />
					Pause auto-scroll
				{:else}
					<Play class="size-4" />
					Resume auto-scroll
				{/if}
			</Button>
		</div>

		<Tabs.Root bind:value={activeLogFile}>
			<Tabs.List>
				<Tabs.Trigger value="LogOutput.log">LogOutput.log</Tabs.Trigger>
				<Tabs.Trigger value="ErrorLog.log">ErrorLog.log</Tabs.Trigger>
			</Tabs.List>
			<Tabs.Content value={activeLogFile} class="mt-3">
				<ScrollArea
					viewportRef={logContainer}
					class="h-96 rounded-md border border-border/70 bg-black/70 p-3 font-mono text-xs leading-5"
				>
					{#if logQuery.isPending}
						<p class="text-muted-foreground">Loading logs...</p>
					{:else if logQuery.isError}
						<p class="text-red-400">Failed to load logs.</p>
					{:else if displayedLines.length === 0}
						<p class="text-muted-foreground">No log output yet.</p>
					{:else}
						{#each displayedLines as line (line.id)}
							<p class="wrap-break-word whitespace-pre-wrap {levelToTextClass(line.level)}">
								{line.text}
							</p>
						{/each}
					{/if}
				</ScrollArea>
			</Tabs.Content>
		</Tabs.Root>
	</Card.Content>
</Card.Root>
