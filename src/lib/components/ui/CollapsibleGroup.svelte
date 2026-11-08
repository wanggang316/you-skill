<script lang="ts">
  import type { Snippet } from "svelte";
  import { ChevronRight } from "@lucide/svelte";
  import { prefersReducedMotion } from "svelte/motion";
  import { slide } from "svelte/transition";
  import { ClickMotion } from "./collapsible.svelte";

  /**
   * A group header that toggles its content. The chevron follows the title and shows only on
   * hover or keyboard focus.
   */
  let {
    open,
    onToggle,
    label,
    title,
    class: className = "",
    buttonClass = "",
    labelClass = "",
    leading,
    trailing,
    actions,
    children,
  }: {
    open: boolean;
    onToggle: () => void;
    label: string;
    title?: string;
    /** Classes of the header row, which holds the toggle button and the actions. */
    class?: string;
    /** Classes of the toggle button: text style and the padding of the click target. */
    buttonClass?: string;
    labelClass?: string;
    /** Icon in front of the title. */
    leading?: Snippet;
    /** Counts or status at the right end of the toggle button. */
    trailing?: Snippet;
    /** Controls in the header row outside the toggle button, such as a menu. */
    actions?: Snippet;
    children: Snippet;
  } = $props();

  const motion = new ClickMotion(() => open);
  const duration = $derived(motion.animate && !prefersReducedMotion.current ? 150 : 0);
</script>

<div class={`group/header flex items-center ${className}`}>
  <button
    class={`group/toggle flex min-w-0 flex-1 items-center gap-1.5 text-left transition ${buttonClass}`}
    type="button"
    onclick={() => motion.click(onToggle)}
    aria-expanded={open}
    {title}
  >
    {@render leading?.()}
    <span class="flex min-w-0 items-center gap-1">
      <span class={`truncate ${labelClass}`}>{label}</span>
      <ChevronRight
        size={12}
        aria-hidden="true"
        class={`shrink-0 opacity-0 transition group-hover/header:opacity-100 group-focus-visible/toggle:opacity-100 ${
          open ? "rotate-90" : ""
        }`}
      />
    </span>
    {#if trailing}
      <span class="ml-auto flex shrink-0 items-center gap-1 pl-1.5">{@render trailing()}</span>
    {/if}
  </button>
  {@render actions?.()}
</div>
{#if open}
  <div
    transition:slide={{ duration }}
    onintroend={() => motion.done()}
    onoutroend={() => motion.done()}
  >
    {@render children()}
  </div>
{/if}
