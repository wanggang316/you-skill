<script>
  import { t } from "../i18n";

  let { agents = [], selectedIds = $bindable([]) } = $props();

  function toggleAgent(agentId) {
    if (selectedIds.includes(agentId)) {
      selectedIds = selectedIds.filter((id) => id !== agentId);
    } else {
      selectedIds = [...selectedIds, agentId];
    }
  }

  function toggleSelectAll() {
    if (selectedIds.length === agents.length) {
      selectedIds = [];
    } else {
      selectedIds = agents.map((a) => a.id);
    }
  }

  const allSelected = $derived(selectedIds.length === agents.length && agents.length > 0);
</script>

<!-- Select All -->
<div class="text-base-content-muted mb-3 ml-2 flex items-center justify-between text-[13px] ">
  <label class="inline-flex items-center gap-2">
    <input type="checkbox" checked={allSelected} onchange={toggleSelectAll} />
    {$t("selectAgent.selectAll")}
  </label>
</div>

<!-- Agent List -->
<div class="mt-3 flex flex-wrap gap-2">
  {#each agents as agent}
    <label
      class="bg-base-200 text-base-content hover:bg-base-300 inline-flex cursor-pointer items-center gap-2 rounded-lg px-2 py-1 text-[13px] transition"
    >
      <input
        type="checkbox"
        checked={selectedIds.includes(agent.id)}
        onchange={() => toggleAgent(agent.id)}
      />
      <span>{agent.display_name}</span>
    </label>
  {/each}
</div>
