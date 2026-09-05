<script lang="ts">
  import AgentAppIcon from "./AgentAppIcon.svelte";
  import { agentNames, resolveAgents } from "../agents";
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
    /** Extra tooltip lines shown below the agent names. */
    title?: string;
  } = $props();

  const members = $derived(resolveAgents(agentIds, agents));
  const primary = $derived(members[0]);
  const tooltip = $derived(
    [primary?.name, members.length > 1 ? agentNames(members.slice(1)) : "", title]
      .filter(Boolean)
      .join("\n")
  );
</script>

{#if primary}
  <span class="inline-flex" title={tooltip}>
    <AgentAppIcon agentId={primary.id} name={primary.name} {size} />
  </span>
{/if}
