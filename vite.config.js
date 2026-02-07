import { defineConfig } from 'vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'
import { nodePolyfills } from 'vite-plugin-node-polyfills'

// https://vite.dev/config/
export default defineConfig({
  plugins: [
    svelte(),
    nodePolyfills({
      include: ['buffer'],
    }),
  ],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
  },
})
