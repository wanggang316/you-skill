<script>
  import { Check, Loader2 } from "@lucide/svelte";
  import { t } from "../i18n";
  import Modal from "./ui/Modal.svelte";

  let {
    open = $bindable(false),
    title = "",
    agents = [],
    initialSelection = [],
    onConfirm = () => {},
    onCancel = () => {},
  } = $props();

  let selectedAgents = $state([]);
  let isInstalling = $state(false);

  // Reset state when modal opens
  $effect(() => {
    if (open) {
      // Use initialSelection if provided, otherwise select all
      selectedAgents =
        initialSelection.length > 0 ? [...initialSelection] : agents.map((a) => a.id);
    }
  });

  function closeModal() {
    open = false;
    onCancel();
  }

  function toggleAgent(agentId) {
    if (selectedAgents.includes(agentId)) {
      selectedAgents = selectedAgents.filter((id) => id !== agentId);
    } else {
      selectedAgents = [...selectedAgents, agentId];
    }
  }

  function toggleSelectAll() {
    if (selectedAgents.length === agents.length) {
      selectedAgents = [];
    } else {
      selectedAgents = agents.map((a) => a.id);
    }
  }

  async function handleConfirm() {
    if (selectedAgents.length === 0) return;
    isInstalling = true;
    try {
      await onConfirm(selectedAgents);
      closeModal();
    } finally {
      isInstalling = false;
    }
  }

  const allSelected = $derived(selectedAgents.length === agents.length && agents.length > 0);
  const hasSelection = $derived(selectedAgents.length > 0);
</script>

<Modal bind:open title={title || $t("selectAgent.defaultTitle")} onClose={closeModal}>
  <div class="flex h-full max-h-[90vh] w-full max-w-md flex-col">
    <!-- Content -->
    <div class="flex-1 overflow-y-auto p-6 pt-16">
      <p class="text-base-content-muted mb-4 text-sm">
        {$t("selectAgent.description")}
      </p>

      <!-- Select All -->
      <div class="text-base-content-muted mb-3 flex items-center justify-between text-xs">
        <label class="inline-flex items-center gap-2">
          <input type="checkbox" checked={allSelected} onchange={toggleSelectAll} />
          {$t("selectAgent.selectAll")}
        </label>
      </div>

      <!-- Agent List - 复用 Local Skills 的布局样式 -->
      <div class="mt-3 flex flex-wrap gap-2">
        {#each agents as agent}
          <label
            class="bg-base-200 text-base-content hover:bg-base-300 inline-flex cursor-pointer items-center gap-3 rounded-lg px-3 py-2 text-sm transition"
          >
            <input
              type="checkbox"
              checked={selectedAgents.includes(agent.id)}
              onchange={() => toggleAgent(agent.id)}
            />
            <span>{agent.display_name}</span>
          </label>
        {/each}
      </div>
    </div>

    <!-- Footer -->
    <div
      class="border-base-300 bg-base-100 flex items-center justify-between rounded-b-2xl border-t px-6 py-4"
    >
      <button
        class="text-base-content-muted hover:text-base-content text-sm transition"
        onclick={closeModal}
        disabled={isInstalling}
        type="button"
      >
        {$t("selectAgent.cancel")}
      </button>
      <button
        class="bg-primary text-primary-content hover:bg-primary-hover rounded-xl px-6 py-2 text-sm transition disabled:opacity-50"
        onclick={handleConfirm}
        disabled={!hasSelection || isInstalling}
        type="button"
      >
        {#if isInstalling}
          <Loader2 size={16} class="mr-1 inline animate-spin" />
        {:else}
          <Check size={16} class="mr-1 inline" />
        {/if}
        {$t("selectAgent.confirm")}
      </button>
    </div>
  </div>
</Modal>
