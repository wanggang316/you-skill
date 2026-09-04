<script lang="ts">
  import { ChevronDown } from "@lucide/svelte";
  import AgentAppIcon from "./AgentAppIcon.svelte";
  import { t } from "../i18n";
  import { agentNames, groupAgents } from "../agents";
  import type { InstallScope } from "../api/hub";
  import type { AgentInfo } from "../api/skills";

  let {
    agents = [],
    scope = "user",
    selectedIds = $bindable<string[]>([]),
    disabled = false,
  }: {
    agents?: AgentInfo[];
    /** Agents are grouped by the directory they read in this scope. */
    scope?: InstallScope;
    selectedIds?: string[];
    disabled?: boolean;
  } = $props();

  let expanded = $state<string[]>([]);

  const groups = $derived(groupAgents(agents, scope));
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

  function toggleExpanded(key: string) {
    expanded = expanded.includes(key)
      ? expanded.filter((item) => item !== key)
      : [...expanded, key];
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
  <div class="space-y-1.5">
    {#each groups as group (group.key)}
      {@const ids = group.agents.map((agent) => agent.id)}
      {@const checked = isSelected(ids)}
      {@const open = expanded.includes(group.key)}
      {@const shared = group.agents.length > 1}
      <div
        class={`rounded-xl border transition ${
          checked
            ? "border-primary/60 bg-primary/10"
            : "border-base-300 bg-base-100 hover:bg-base-200"
        } ${disabled ? "opacity-60" : ""}`}
      >
        <div class="flex items-center gap-2 px-2.5 py-2">
          <button
            class="flex min-w-0 flex-1 items-center gap-2 text-left"
            type="button"
            {disabled}
            onclick={() => toggleGroup(ids)}
            aria-pressed={checked}
          >
            <input type="checkbox" class="accent-primary" {checked} {disabled} tabindex="-1" />
            <span class="flex shrink-0 items-center">
              {#each group.agents.slice(0, shared && !open ? 3 : group.agents.length) as agent, index (agent.id)}
                <span class={index > 0 ? (open ? "ml-1" : "-ml-2.5") : ""}>
                  <AgentAppIcon agentId={agent.id} name={agent.display_name} size="sm" />
                </span>
              {/each}
            </span>
            <span class="min-w-0 flex-1">
              <span class="text-base-content block truncate text-[13px]">
                {shared
                  ? $t("install.sharedAgents", { count: group.agents.length })
                  : group.agents[0].display_name}
              </span>
              <span class="text-base-content-faint block truncate text-[11px]" title={group.path}>
                {group.path}
              </span>
            </span>
          </button>
          {#if shared}
            <button
              class="text-base-content-subtle hover:text-base-content shrink-0 transition"
              type="button"
              onclick={() => toggleExpanded(group.key)}
              title={agentNames(group.agents)}
              aria-expanded={open}
            >
              <ChevronDown size={15} class={open ? "rotate-180 transition" : "transition"} />
            </button>
          {/if}
        </div>
        {#if shared && open}
          <p class="text-base-content-muted px-2.5 pb-2 text-[11px]">
            {agentNames(group.agents)}
          </p>
        {/if}
      </div>
    {/each}
  </div>
</div>
