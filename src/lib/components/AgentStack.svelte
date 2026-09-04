<script lang="ts">
  import AgentAppIcon from "./AgentAppIcon.svelte";
  import type { AgentInfo } from "../api/skills";

  let {
    agentIds = [],
    agents,
    size = "sm",
    title = "",
  }: {
    agentIds?: string[];
    agents: Map<string, AgentInfo>;
    size?: "sm" | "md";
    /** Extra tooltip line above the agent names. */
    title?: string;
  } = $props();

  /** Icons shown while collapsed; the rest are counted in a badge. */
  const MAX_COLLAPSED = 4;

  let expanded = $state(false);

  const items = $derived(agentIds.map((id) => ({ id, name: agents.get(id)?.display_name ?? id })));
  const visible = $derived(expanded ? items : items.slice(0, MAX_COLLAPSED));
  const hidden = $derived(items.length - visible.length);
  const tooltip = $derived(
    [title, items.map((item) => item.name).join(", ")].filter(Boolean).join("\n")
  );
</script>

<button
  class="inline-flex items-center"
  type="button"
  title={tooltip}
  aria-expanded={expanded}
  onclick={(event) => {
    event.stopPropagation();
    if (items.length > 1) expanded = !expanded;
  }}
>
  {#each visible as item, index (item.id)}
    <span
      class="agent-stack-item"
      class:stacked={!expanded && index > 0}
      class:spread={expanded && index > 0}
      style={`z-index: ${items.length - index}`}
    >
      <AgentAppIcon agentId={item.id} name={item.name} {size} />
    </span>
  {/each}
  {#if hidden > 0}
    <span class="text-base-content-subtle ml-1 text-[11px]">+{hidden}</span>
  {/if}
</button>

<style>
  .agent-stack-item {
    display: inline-flex;
    transition: margin-left 0.18s ease;
  }

  .agent-stack-item.stacked {
    margin-left: -0.7rem;
  }

  .agent-stack-item.spread {
    margin-left: 0.25rem;
  }
</style>
