<script lang="ts">
  import AgentAppIcon from "./AgentAppIcon.svelte";
  import { resolveAgents } from "../agents";
  import type { AgentInfo } from "../api/skills";

  let {
    agentIds = [],
    agents,
    size = "sm",
    path = "",
    note = "",
  }: {
    agentIds?: string[];
    agents: Map<string, AgentInfo>;
    size?: "sm" | "md";
    /** Directory or file the badge stands for, shown in the hover card. */
    path?: string;
    /** One extra line for the hover card, e.g. "copy · in sync". */
    note?: string;
  } = $props();

  /** Hover card width in px; must match the `w-80` class below. */
  const CARD_WIDTH = 320;
  const CARD_GAP = 6;
  const OPEN_DELAY_MS = 150;
  const VIEWPORT_MARGIN = 8;

  const members = $derived(resolveAgents(agentIds, agents));
  const primary = $derived(members[0]);
  const others = $derived(members.slice(1));

  let anchor = $state<HTMLElement | null>(null);
  let visible = $state(false);
  let placeAbove = $state(false);
  let left = $state(0);
  let offset = $state(0);
  let timer: ReturnType<typeof setTimeout> | null = null;

  function show() {
    if (timer) clearTimeout(timer);
    timer = setTimeout(() => {
      timer = null;
      if (!anchor) return;
      const rect = anchor.getBoundingClientRect();
      left = Math.max(
        VIEWPORT_MARGIN,
        Math.min(rect.left, window.innerWidth - CARD_WIDTH - VIEWPORT_MARGIN)
      );
      // Flip above the badge when the lower half of the window is too short for the card.
      placeAbove = rect.bottom > window.innerHeight * 0.65;
      offset = placeAbove ? window.innerHeight - rect.top + CARD_GAP : rect.bottom + CARD_GAP;
      visible = true;
    }, OPEN_DELAY_MS);
  }

  function hide() {
    if (timer) clearTimeout(timer);
    timer = null;
    visible = false;
  }

  // The card is fixed to the viewport, so any scroll would leave it behind its badge.
  $effect(() => {
    if (!visible) return;
    window.addEventListener("scroll", hide, true);
    return () => window.removeEventListener("scroll", hide, true);
  });

  /** Render the card under <body> so overflow clipping and modal transforms cannot affect it. */
  function portal(node: HTMLElement) {
    document.body.appendChild(node);
    return {
      destroy() {
        node.remove();
      },
    };
  }
</script>

{#if primary}
  <span
    class="inline-flex"
    role="img"
    aria-label={primary.name}
    bind:this={anchor}
    onmouseenter={show}
    onmouseleave={hide}
    onfocusin={show}
    onfocusout={hide}
  >
    <AgentAppIcon agentId={primary.id} name={primary.name} {size} />
  </span>

  {#if visible}
    <div
      use:portal
      role="tooltip"
      class="bg-base-100 border-base-300 text-base-content pointer-events-none fixed z-[10020] w-80 max-w-[calc(100vw-1rem)] rounded-xl border p-3 shadow-lg"
      style={`left:${left}px; ${placeAbove ? "bottom" : "top"}:${offset}px;`}
    >
      <div class="flex min-w-0 items-center gap-2">
        <AgentAppIcon agentId={primary.id} name={primary.name} size="sm" />
        <span class="truncate text-[13px] font-medium">{primary.name}</span>
      </div>
      {#if others.length > 0}
        <div class="mt-2 flex flex-wrap gap-1">
          {#each others as agent (agent.id)}
            <span
              class="bg-base-200 text-base-content-muted inline-flex items-center gap-1 rounded-md py-0.5 pr-1.5 pl-0.5 text-[11px]"
            >
              <AgentAppIcon agentId={agent.id} name={agent.name} size="sm" />
              {agent.name}
            </span>
          {/each}
        </div>
      {/if}
      {#if path}
        <p class="text-base-content-subtle mt-2 text-[11px] leading-snug break-all">{path}</p>
      {/if}
      {#if note}
        <p class="text-base-content-muted mt-1 text-[11px]">{note}</p>
      {/if}
    </div>
  {/if}
{/if}
