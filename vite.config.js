import { defineConfig } from "vite";
import { sveltekit } from "@sveltejs/kit/vite";
import { nodePolyfills } from "vite-plugin-node-polyfills";
import tailwindcss from "@tailwindcss/vite";

// https://kit.svelte.dev/docs/config/
export default defineConfig({
  plugins: [
    sveltekit(),
    nodePolyfills({
      include: ["buffer"],
    }),
    tailwindcss(),
  ],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
  },
});
