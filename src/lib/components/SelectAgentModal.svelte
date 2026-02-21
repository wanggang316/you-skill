<script>
  import { get } from "svelte/store";
  import { t } from "../i18n";
  import Modal from "./ui/Modal.svelte";
  import AgentSelector from "./AgentSelector.svelte";
  import PrimaryActionButton from "./ui/PrimaryActionButton.svelte";
  import { settings } from "../stores/settings";

  let {
    open = $bindable(false),
    title = "",
    confirmText = "",
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
      selectedMethod = get(settings).sync_mode || "symlink";
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
  const finalConfirmText = $derived(confirmText || $t("selectAgent.confirm"));
</script>

<Modal
  bind:open
  title={title || $t("selectAgent.defaultTitle")}
  onClose={closeModal}
  containerClass="max-w-lg min-w-md"
>
  <div class="w-full px-6 pt-2 pb-6">
    <p class="text-base-content-muted mb-4 text-sm">
      {$t("selectAgent.description")}
    </p>

    <!-- Agent List -->
    <AgentSelector {agents} bind:selectedIds={selectedAgents} />
  </div>
  {#snippet footer()}
    <select
      bind:value={selectedMethod}
      class="bg-base-100 text-base-content mr-3 rounded-lg px-3 py-2 text-sm"
    >
      <option value="symlink">{$t("settings.syncMode.symlink")}</option>
      <option value="copy">{$t("settings.syncMode.copy")}</option>
    </select>
    <PrimaryActionButton
      onclick={handleConfirm}
      disabled={!hasSelection || isInstalling}
      loading={isInstalling}
      loadingText={finalConfirmText}
      className="px-6"
    >
      {finalConfirmText}
    </PrimaryActionButton>
  {/snippet}
</Modal>
