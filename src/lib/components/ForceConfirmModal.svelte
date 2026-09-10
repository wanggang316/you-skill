<script lang="ts">
  import { AlertTriangle } from "@lucide/svelte";
  import Modal from "$lib/components/ui/Modal.svelte";
  import PrimaryActionButton from "$lib/components/ui/PrimaryActionButton.svelte";
  import { t } from "../i18n";
  import { closeForceModal, forceModal } from "../stores/modals";

  let open = $state(false);
  let busy = $state(false);

  $effect(() => {
    open = $forceModal.open;
  });

  function handleCancel() {
    const { onCancel } = $forceModal;
    closeForceModal();
    open = false;
    onCancel?.();
  }

  async function handleConfirm() {
    const { onConfirm } = $forceModal;
    busy = true;
    try {
      await onConfirm?.();
    } finally {
      busy = false;
      closeForceModal();
      open = false;
    }
  }
</script>

<Modal bind:open title={$t("drift.force.title")} onClose={handleCancel} containerClass="max-w-md">
  <div class="space-y-3 px-6 pt-2 pb-6 text-sm">
    <div class="text-warning-content flex items-start gap-2">
      <AlertTriangle size={18} class="mt-0.5 shrink-0" />
      <p class="text-base-content">{$t("drift.force.description")}</p>
    </div>
    <ul class="bg-base-200 space-y-1 rounded-xl p-3 text-xs">
      {#each $forceModal.blockers as blocker}
        <li class="text-base-content-muted break-all">{blocker}</li>
      {/each}
    </ul>
  </div>
  {#snippet footer()}
    <button
      class="border-base-300 text-base-content hover:bg-base-200 rounded-xl border px-4 py-2 text-sm transition"
      type="button"
      onclick={handleCancel}
      disabled={busy}
    >
      {$t("common.cancel")}
    </button>
    <PrimaryActionButton
      onclick={handleConfirm}
      loading={busy}
      className="bg-error hover:bg-error/85 text-error-content"
    >
      {$t("drift.force.confirm")}
    </PrimaryActionButton>
  {/snippet}
</Modal>
