<script>
  import { Check, Loader2 } from "@lucide/svelte";
  import { t } from "../i18n";
  import Modal from "./ui/Modal.svelte";
  import AgentSelector from "./AgentSelector.svelte";

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

  const hasSelection = $derived(selectedAgents.length > 0);
</script>

<Modal bind:open title={title || $t("selectAgent.defaultTitle")} onClose={closeModal}>
  <div class="flex h-full max-h-[90vh] w-full max-w-md flex-col">
    <!-- Content -->
    <div class="flex-1 overflow-y-auto p-6 pt-16">
      <p class="text-base-content-muted mb-4 text-sm">
        {$t("selectAgent.description")}
      </p>

      <!-- Agent List -->
      <AgentSelector {agents} selectedIds={selectedAgents} />
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
