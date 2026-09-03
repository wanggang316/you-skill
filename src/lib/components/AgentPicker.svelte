<script lang="ts">
  import AgentAppIcon from "./AgentAppIcon.svelte";
  import { t } from "../i18n";
  import type { AgentInfo } from "../api/skills";

  let {
    agents = [],
    selectedIds = $bindable<string[]>([]),
    disabled = false,
  }: {
    agents?: AgentInfo[];
    selectedIds?: string[];
    disabled?: boolean;
  } = $props();

  const allSelected = $derived(agents.length > 0 && selectedIds.length === agents.length);

  function toggle(id: string) {
    if (disabled) return;
    selectedIds = selectedIds.includes(id)
      ? selectedIds.filter((item) => item !== id)
      : [...selectedIds, id];
  }

  function toggleAll() {
    if (disabled) return;
    selectedIds = allSelected ? [] : agents.map((agent) => agent.id);
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
  <div class="grid grid-cols-2 gap-2 sm:grid-cols-3">
    {#each agents as agent (agent.id)}
      {@const checked = selectedIds.includes(agent.id)}
      <label
        class={`flex cursor-pointer items-center gap-2 rounded-xl border px-2.5 py-2 text-[13px] transition ${
          checked
            ? "border-primary/60 bg-primary/10 text-base-content"
            : "border-base-300 bg-base-100 text-base-content-muted hover:bg-base-200"
        } ${disabled ? "cursor-not-allowed opacity-60" : ""}`}
      >
        <input
          type="checkbox"
          class="sr-only"
          {checked}
          onchange={() => toggle(agent.id)}
          {disabled}
        />
        <AgentAppIcon agentId={agent.id} name={agent.display_name} />
        <span class="min-w-0 flex-1 truncate">{agent.display_name}</span>
      </label>
    {/each}
  </div>
</div>
