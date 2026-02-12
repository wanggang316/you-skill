<script lang="ts">
  import "../app.css";
  import { onMount } from "svelte";
  import { browser } from "$app/environment";
  import { loadSettings } from "../lib/stores/settings";
  import { check } from "@tauri-apps/plugin-updater";

  let { children } = $props();

  onMount(() => {
    if (!browser) {
      return () => {};
    }

    const allowedThemes = new Set<string>(["light", "dark", "system"]);
    const savedTheme = localStorage.getItem("theme");
    if (savedTheme && allowedThemes.has(savedTheme)) {
      // You can set theme via store or CSS variables
      // For now, we'll use the existing settings store
    } else {
      localStorage.setItem("theme", "system");
    }

    // Load settings and check for updates - 不阻塞渲染
    loadSettings().catch(console.error);
    checkForUpdate().catch(console.error);

    return () => {};
  });

  const checkForUpdate = async () => {
    try {
      const update = await check();
      if (update) {
        // Dispatch event for child components to listen to
        window.dispatchEvent(new CustomEvent("app:has-update", { detail: true }));
      }
    } catch (error) {
      console.error("Failed to check for update:", error);
    }
  };
</script>

{@render children()}
