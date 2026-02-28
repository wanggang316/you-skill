<script lang="ts">
  import "../app.css";
  import { onMount } from "svelte";
  import { browser } from "$app/environment";
  import { loadSettings } from "$lib/stores/settings";
  import { ensureUpdateChecked } from "$lib/stores/updater";

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
    ensureUpdateChecked().catch(console.error);

    return () => {};
  });
</script>

{@render children()}
