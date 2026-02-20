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
    onConfirm = async () => true,
    onCancel = () => {},
  } = $props();

  /** @type {string[]} */
  let selectedAgents = $state([]);
  /** @type {"symlink" | "copy"} */
  let selectedMethod = $state("symlink");
  let isInstalling = $state(false);

  // Reset state when modal opens
  $effect(() => {
    if (open) {
      // Use initialSelection if provided, otherwise select all
      selectedAgents =
        initialSelection.length > 0 ? [...initialSelection] : agents.map((a) => a.id);
      selectedMethod = "symlink";
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
      const shouldClose = await onConfirm(selectedAgents, selectedMethod);
      if (shouldClose !== false) {
        closeModal();
      }
    } finally {
      isInstalling = false;
    }
  }

  const hasSelection = $derived(selectedAgents.length > 0);
</script>

<Modal bind:open title={title || $t("selectAgent.defaultTitle")} onClose={closeModal}>
  <div class="flex h-full w-full max-w-md flex-col">
    <!-- Content -->
    <div class="flex-1 overflow-y-auto px-6 pt-2 pb-6">
      <p class="text-base-content-muted mb-4 text-sm">
        {$t("selectAgent.description")}
      </p>

      <!-- Agent List -->
      <AgentSelector {agents} bind:selectedIds={selectedAgents} />
    </div>

    <!-- Footer -->
    <div
      class="border-base-300 bg-base-100 flex items-center justify-end rounded-b-2xl border-t px-6 py-3"
    >
      <select
        bind:value={selectedMethod}
        class="bg-base-100 text-base-content mr-3 rounded-lg px-3 py-2 text-sm"
      >
        <option value="symlink">{$t("settings.syncMode.symlink")}</option>
        <option value="copy">{$t("settings.syncMode.copy")}</option>
      </select>
      <button
        class="bg-primary text-primary-content hover:bg-primary-hover rounded-xl px-6 py-2 text-sm transition disabled:opacity-50"
        onclick={handleConfirm}
        disabled={!hasSelection || isInstalling}
        type="button"
      >
        {#if isInstalling}
          <Loader2 size={16} class="mr-1 inline animate-spin" />
        {/if}
        {$t("selectAgent.confirm")}
      </button>
    </div>
  </div>
</Modal>
