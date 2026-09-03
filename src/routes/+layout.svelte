<script lang="ts">
  import "../app.css";
  import { onMount } from "svelte";
  import { browser } from "$app/environment";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { loadSettings } from "$lib/stores/settings";
  import { ensureUpdateChecked } from "$lib/stores/updater";

  let { children } = $props();

  const dragExcludedSelector = [
    "a",
    "button",
    "input",
    "select",
    "textarea",
    "[contenteditable='true']",
    "[data-window-drag-exclude]",
  ].join(",");

  const handleWindowDrag = (event: MouseEvent) => {
    if (event.button !== 0 || event.buttons !== 1) return;

    const target = event.target;
    if (!(target instanceof Element)) return;
    if (!target.closest("[data-window-drag-region]")) return;
    if (target.closest(dragExcludedSelector)) return;

    getCurrentWindow()
      .startDragging()
      .catch((error) => {
        console.error("Failed to start window dragging:", error);
      });
  };

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
    document.addEventListener("mousedown", handleWindowDrag);

    return () => {
      document.removeEventListener("mousedown", handleWindowDrag);
    };
  });
</script>

{@render children()}
