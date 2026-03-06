import svelte from 'eslint-plugin-svelte';
import { defineConfig } from 'eslint/config';
import ts from 'typescript-eslint';
import svelteConfig from './svelte.config.js';

// Minimal ESLint config: only Svelte-file concerns that oxlint does not cover.
export default defineConfig(
	{
		ignores: ['node_modules/', '.svelte-kit/', 'build/', 'static/', 'src-tauri/']
	},
	...svelte.configs.recommended,
	{
		files: ['**/*.svelte'],

		languageOptions: {
			parserOptions: {
				projectService: true,
				extraFileExtensions: ['.svelte'],
				parser: ts.parser,
				svelteConfig
			}
		},

		rules: {
			'no-unused-vars': 'off',
			'no-undef': 'off',
			'svelte/no-navigation-without-resolve': 'off'
		}
	}
);
