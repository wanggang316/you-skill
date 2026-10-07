<script lang="ts">
  import { Tooltip } from "bits-ui";
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

  const members = $derived(resolveAgents(agentIds, agents));
  const primary = $derived(members[0]);
  const others = $derived(members.slice(1));
</script>

{#if primary}
  <Tooltip.Root>
    <Tooltip.Trigger>
      {#snippet child({ props })}
        <span {...props} class="inline-flex" role="img" aria-label={primary.name}>
          <AgentAppIcon agentId={primary.id} name={primary.name} {size} />
        </span>
      {/snippet}
    </Tooltip.Trigger>
    <Tooltip.Portal>
      <Tooltip.Content
        class="bg-base-100 border-base-300 text-base-content pointer-events-none z-(--z-popover) w-80 max-w-[calc(100vw-1rem)] rounded-xl border p-3 shadow-lg"
        side="bottom"
        align="start"
        sideOffset={6}
        collisionPadding={8}
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
      </Tooltip.Content>
    </Tooltip.Portal>
  </Tooltip.Root>
{/if}
