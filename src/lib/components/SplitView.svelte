<script lang="ts">
  import type { Snippet } from "svelte";
  import { t } from "$lib/i18n";
  import {
    clampListPaneWidth,
    LIST_PANE,
    listPaneWidth,
    saveListPaneWidth,
  } from "$lib/navigation/list-pane";

  /** Two grid children: the list column, then the content panel. */
  let { children }: { children: Snippet } = $props();

  let container = $state<HTMLElement | null>(null);
  let resizing = $state(false);

  function startResize(event: PointerEvent) {
    if (event.button !== 0 || !container) return;
    event.preventDefault();
    resizing = true;
    const handle = event.currentTarget as HTMLElement;
    const left = container.getBoundingClientRect().left;
    handle.setPointerCapture(event.pointerId);
    const onMove = (move: PointerEvent) =>
      listPaneWidth.set(clampListPaneWidth(move.clientX - left));
    const onUp = () => {
      resizing = false;
      handle.releasePointerCapture(event.pointerId);
      handle.removeEventListener("pointermove", onMove);
      handle.removeEventListener("pointerup", onUp);
      handle.removeEventListener("pointercancel", onUp);
      saveListPaneWidth($listPaneWidth);
    };
    handle.addEventListener("pointermove", onMove);
    handle.addEventListener("pointerup", onUp);
    handle.addEventListener("pointercancel", onUp);
  }
</script>

<div
  bind:this={container}
  class={`relative grid min-h-0 flex-1 ${resizing ? "cursor-col-resize select-none" : ""}`}
  style={`grid-template-columns: ${$listPaneWidth}px minmax(0, 1fr);`}
>
  {@render children()}

  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="absolute top-0 bottom-0 z-20 w-2 -translate-x-1/2 cursor-col-resize"
    style={`left: ${$listPaneWidth}px;`}
    title={$t("splitView.resize")}
    onpointerdown={startResize}
    ondblclick={() => saveListPaneWidth(LIST_PANE.initial)}
  ></div>
</div>
