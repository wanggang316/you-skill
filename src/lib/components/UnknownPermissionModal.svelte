<script lang="ts">
  import Modal from "./ui/Modal.svelte";
  import PrimaryActionButton from "./ui/PrimaryActionButton.svelte";
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

<Modal
  bind:open
  title={$t("local.unknown.confirmTitle")}
  onClose={closeModal}
  containerClass="max-w-sm"
>
  <div class="w-full px-6 pt-6 pb-6">
    <p class="text-base-content text-sm">
      {$t("local.unknown.confirmManage", { name: skillName })}
    </p>
  </div>
  {#snippet footer()}
    <label class="text-base-content-subtle mr-1 inline-flex items-center gap-2 text-xs">
      <input type="checkbox" bind:checked={rememberChoice} />
      {$t("local.unknown.rememberChoice")}
    </label>
    <PrimaryActionButton onclick={handleConfirm}>
      {$t("selectAgent.confirm")}
    </PrimaryActionButton>
  {/snippet}
</Modal>
