import tailwindcss from "@tailwindcss/vite";
import { sveltekit } from "@sveltejs/kit/vite";
import { defineConfig } from "vite";
import svg from "@poppanator/sveltekit-svg";

export default defineConfig({
  plugins: [tailwindcss(), sveltekit(), svg()],
});
