<script lang="ts">
  import Modal from "./ui/Modal.svelte";
  import { t } from "../i18n";

  let {
    open = $bindable(false),
    title = "",
    candidates = [],
    onConfirm = () => {},
    onCancel = () => {},
  } = $props<{
    open?: boolean;
    title?: string;
    candidates?: string[];
    onConfirm?: (path: string) => Promise<void> | void;
    onCancel?: () => void;
  }>();

  let selectedPath = $state("");

  $effect(() => {
    if (open) {
      selectedPath = candidates[0] ?? "";
    }
  });

  const closeModal = () => {
    open = false;
    onCancel();
  };

  const handleConfirm = async () => {
    if (!selectedPath) return;
    await onConfirm(selectedPath);
    open = false;
  };
</script>

<Modal bind:open title={title || $t("local.sourceSelect.titleDefault")} onClose={closeModal}>
  <div class="flex h-full max-h-[90vh] w-full max-w-xl flex-col">
    <div class="flex-1 overflow-y-auto px-6 pt-16 pb-6">
      <p class="text-base-content-muted mb-3 text-sm">{$t("local.sourceSelect.description")}</p>
      <div class="space-y-2">
        {#each candidates as path}
          <label
            class="border-base-300 bg-base-100 hover:bg-base-200 flex cursor-pointer items-start gap-3 rounded-lg border px-3 py-2 text-sm"
          >
            <input
              type="radio"
              name="sourcePath"
              value={path}
              checked={selectedPath === path}
              onchange={() => {
                selectedPath = path;
              }}
            />
            <span class="text-base-content break-all">{path}</span>
          </label>
        {/each}
      </div>
    </div>
    <div class="border-base-300 bg-base-100 flex items-center justify-end gap-2 border-t px-6 py-3">
      <button
        class="border-base-300 text-base-content hover:bg-base-200 rounded-xl border px-4 py-2 text-sm transition"
        onclick={closeModal}
        type="button"
      >
        {$t("addSkill.cancel")}
      </button>
      <button
        class="bg-primary text-primary-content hover:bg-primary-hover rounded-xl px-4 py-2 text-sm transition disabled:opacity-50"
        onclick={handleConfirm}
        disabled={!selectedPath}
        type="button"
      >
        {$t("selectAgent.confirm")}
      </button>
    </div>
  </div>
</Modal>
