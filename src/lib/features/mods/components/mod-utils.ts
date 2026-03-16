import { marked } from 'marked';
import { Globe, MessageCircle, Github } from '@lucide/svelte';
import type { Component } from 'svelte';
import type { Mod } from '$lib/features/mods/schema';

export function safeParseMarkdown(content: string | undefined): string {
	if (!content) return '';
	try {
		return marked.parse(content, { async: false });
	} catch {
		return content;
	}
}

export function pickDefaultVersion(
	versions: { version: string; created_at: number }[]
): string | null {
	if (versions.length === 0) return null;
	return [...versions].toSorted((a, b) => b.created_at - a.created_at)[0].version;
}

export function getLinkIcon(type: string): Component {
	switch (type.toLowerCase()) {
		case 'github':
			return Github;
		case 'discord':
			return MessageCircle;
		default:
			return Globe;
	}
}

export function formatLinkType(type: string) {
	return type.charAt(0).toUpperCase() + type.slice(1);
}

export function mapModsById(mods: Array<Mod | undefined>): Map<string, Mod> {
	return new Map(mods.filter((mod): mod is Mod => mod !== undefined).map((mod) => [mod.id, mod]));
}
