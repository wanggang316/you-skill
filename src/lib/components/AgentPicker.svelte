<script lang="ts">
  import AgentAppIcon from "./AgentAppIcon.svelte";
  import { t } from "../i18n";
  import { agentNames, groupAgents, type AgentTarget } from "../agents";
  import type { InstallScope } from "../api/hub";
  import type { AgentInfo } from "../api/skills";

  let {
    agents = [],
    scope = "user",
    target = "skills",
    selectedIds = $bindable<string[]>([]),
    disabled = false,
  }: {
    agents?: AgentInfo[];
    /** Agents are grouped by the location they read in this scope. */
    scope?: InstallScope;
    /** Group by skills directory or by instruction file. */
    target?: AgentTarget;
    selectedIds?: string[];
    disabled?: boolean;
  } = $props();

  const groups = $derived(groupAgents(agents, scope, target));
  const allSelected = $derived(
    groups.length > 0 &&
      groups.every((group) => group.agents.every((a) => selectedIds.includes(a.id)))
  );

  const isSelected = (ids: string[]) => ids.every((id) => selectedIds.includes(id));

  function toggleGroup(ids: string[]) {
    if (disabled) return;
    selectedIds = isSelected(ids)
      ? selectedIds.filter((id) => !ids.includes(id))
      : [...selectedIds, ...ids.filter((id) => !selectedIds.includes(id))];
  }

  function toggleAll() {
    if (disabled) return;
    selectedIds = allSelected ? [] : groups.flatMap((group) => group.agents.map((a) => a.id));
  }
</script>

<div class="space-y-2">
  <div class="text-base-content-muted flex items-center justify-between text-[13px]">
    <span>{$t("install.selectAgents")}</span>
    <label class="inline-flex cursor-pointer items-center gap-2">
      <input type="checkbox" checked={allSelected} onchange={toggleAll} {disabled} />
      {$t("install.selectAll")}
    </label>
  </div>
  <div class="grid grid-cols-2 gap-1.5">
    {#each groups as group (group.key)}
      {@const ids = group.agents.map((agent) => agent.id)}
      {@const checked = isSelected(ids)}
      {@const primary = group.agents[0]}
      <button
        class={`flex w-full items-center gap-2 rounded-xl border px-2.5 py-2 text-left transition ${
          checked
            ? "border-primary/60 bg-primary/10"
            : "border-base-300 bg-base-100 hover:bg-base-200"
        } ${disabled ? "opacity-60" : ""}`}
        type="button"
        {disabled}
        onclick={() => toggleGroup(ids)}
        aria-pressed={checked}
        title={group.agents.length > 1 ? agentNames(group.agents.slice(1)) : undefined}
      >
        <input type="checkbox" class="accent-primary" {checked} {disabled} tabindex="-1" />
        <AgentAppIcon agentId={primary.id} name={primary.display_name} size="sm" />
        <span class="min-w-0 flex-1">
          <span class="text-base-content block truncate text-[13px]">{primary.display_name}</span>
          <span class="text-base-content-faint block truncate text-[11px]" title={group.path}>
            {group.path}
          </span>
        </span>
      </button>
    {/each}
  </div>
</div>
