import { defineConfig } from "vite";
import { sveltekit } from "@sveltejs/kit/vite";
import { nodePolyfills } from "vite-plugin-node-polyfills";
import tailwindcss from "@tailwindcss/vite";
import { fileURLToPath, URL } from "node:url";
import { devPort } from "./scripts/dev-port.mjs";

// https://kit.svelte.dev/docs/config/
export default defineConfig({
  resolve: {
    alias: {
      "@": fileURLToPath(new URL("./src", import.meta.url)),
    },
  },
  plugins: [
    sveltekit(),
    nodePolyfills({
      include: ["buffer"],
    }),
    tailwindcss(),
  ],
  clearScreen: false,
  server: {
    port: devPort(),
    strictPort: true,
  },
});
