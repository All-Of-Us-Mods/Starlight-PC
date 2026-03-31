import tailwindcss from "@tailwindcss/vite";
import { sveltekit } from "@sveltejs/kit/vite";
import { defineConfig } from "vite-plus";
import svg from "@poppanator/sveltekit-svg";

import type { PluginOption } from "vite-plus";

export default defineConfig({
  staged: {
    "*": "vp check --fix",
  },
  fmt: {
    sortTailwindcss: {
      stylesheet: "./src/app.css",
      functions: ["clsx", "cn", "cva"],
    },
    ignorePatterns: ["src-tauri/", ".svelte-kit/", "build/", "static/"],
  },
  lint: {
    options: {
      typeAware: true,
      typeCheck: true,
    },
    ignorePatterns: ["src-tauri/", ".svelte-kit/", "build/", "static/", "src/lib/components/ui"],
  },
  plugins: [tailwindcss(), sveltekit(), svg()] as PluginOption[],
});
