<script lang="ts">
  import Modal from "./ui/Modal.svelte";
  import { t } from "../i18n";

  let {
    open = $bindable(false),
    skillName = "",
    onConfirm = () => {},
    onCancel = () => {},
  } = $props<{
    open?: boolean;
    skillName?: string;
    onConfirm?: (rememberChoice: boolean) => Promise<void> | void;
    onCancel?: () => void;
  }>();

  let rememberChoice = $state(true);

  $effect(() => {
    if (open) {
      rememberChoice = true;
    }
  });

  const closeModal = () => {
    open = false;
    onCancel();
  };

  const handleConfirm = async () => {
    await onConfirm(rememberChoice);
    open = false;
  };
</script>

<Modal bind:open title={$t("local.unknown.confirmTitle")} onClose={closeModal}>
  <div class="flex h-full max-h-[90vh] w-full max-w-lg flex-col">
    <div class="flex-1 overflow-y-auto px-6 pt-16 pb-6">
      <p class="text-base-content text-sm">
        {$t("local.unknown.confirmManage", { name: skillName })}
      </p>
    </div>
    <div
      class="border-base-300 bg-base-100 flex items-center justify-end gap-3 rounded-b-2xl border-t px-6 py-3"
    >
      <label class="text-base-content-subtle mr-1 inline-flex items-center gap-2 text-xs">
        <input type="checkbox" bind:checked={rememberChoice} />
        {$t("local.unknown.rememberChoice")}
      </label>
      <button
        class="bg-primary text-primary-content hover:bg-primary-hover rounded-xl px-4 py-2 text-sm transition"
        onclick={handleConfirm}
        type="button"
      >
        {$t("selectAgent.confirm")}
      </button>
    </div>
  </div>
</Modal>
