import svelte from 'eslint-plugin-svelte';
import { defineConfig } from 'eslint/config';
import ts from 'typescript-eslint';
import svelteConfig from './svelte.config.js';

// Minimal ESLint config — only for Svelte-specific rules that oxlint
// doesn't support yet. All JS/TS linting is handled by oxlint.
export default defineConfig(
	{
		ignores: ['node_modules/', '.svelte-kit/', 'build/', 'static/', 'src-tauri/']
	},
	...svelte.configs.recommended,
	{
		files: ['**/*.svelte', '**/*.svelte.ts', '**/*.svelte.js'],

		languageOptions: {
			parserOptions: {
				projectService: true,
				extraFileExtensions: ['.svelte'],
				parser: ts.parser,
				svelteConfig
			}
		},

		rules: {
			// Disable rules that oxlint already covers
			'no-unused-vars': 'off',
			'no-undef': 'off',

			// Svelte-specific rules
			'svelte/no-navigation-without-resolve': 'off'
		}
	}
);
