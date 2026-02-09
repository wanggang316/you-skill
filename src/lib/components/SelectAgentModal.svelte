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
      selectedAgents = initialSelection.length > 0
        ? [...initialSelection]
        : agents.map((a) => a.id);
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

  const allSelected = $derived(
    selectedAgents.length === agents.length && agents.length > 0,
  );
  const hasSelection = $derived(selectedAgents.length > 0);
</script>

<Modal
  bind:open={open}
  title={title || $t("selectAgent.defaultTitle")}
  onClose={closeModal}
>
  <div class="flex h-full max-h-[90vh] w-full max-w-md flex-col bg-base-100">
    <!-- Content -->
    <div class="flex-1 overflow-y-auto p-6 pt-16">
      <p class="mb-4 text-sm text-[var(--base-content-muted)]">
        {$t("selectAgent.description")}
      </p>

      <!-- Select All -->
      <div
        class="mb-3 flex items-center justify-between text-xs text-[var(--base-content-muted)]"
      >
        <label class="inline-flex items-center gap-2">
          <input
            type="checkbox"
            checked={allSelected}
            onchange={toggleSelectAll}
          />
          {$t("selectAgent.selectAll")}
        </label>
      </div>

      <!-- Agent List - 复用 Local Skills 的布局样式 -->
      <div class="mt-3 flex flex-wrap gap-2">
        {#each agents as agent}
          <label
            class="inline-flex items-center gap-3 rounded-lg bg-[var(--base-200)] px-3 py-2 text-sm text-[var(--base-content)] cursor-pointer transition hover:bg-[var(--base-300)]"
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
      class="flex items-center justify-between border-t border-[var(--base-300)] px-6 py-4 bg-[var(--base-100)] rounded-b-2xl"
    >
      <button
        class="text-sm text-[var(--base-content-muted)] transition hover:text-[var(--base-content)]"
        onclick={closeModal}
        disabled={isInstalling}
        type="button"
      >
        {$t("selectAgent.cancel")}
      </button>
      <button
        class="rounded-xl bg-[var(--primary)] px-6 py-2 text-sm text-[var(--primary-content)] transition hover:bg-[var(--primary-hover)] disabled:opacity-50"
        onclick={handleConfirm}
        disabled={!hasSelection || isInstalling}
        type="button"
      >
        {#if isInstalling}
          <Loader2 size={16} class="animate-spin inline mr-1" />
        {:else}
          <Check size={16} class="inline mr-1" />
        {/if}
        {$t("selectAgent.confirm")}
      </button>
    </div>
  </div>
</Modal>
