<script lang="ts">
  import Modal from "./ui/Modal.svelte";
  import { t } from "../i18n";
  import SkillVersionGroupSelector from "./SkillVersionGroupSelector.svelte";
  import type { SourceVersionGroup } from "../api/skills";

  let {
    open = $bindable(false),
    skillName = "",
    versionGroups = [],
    onConfirm = () => {},
    onCancel = () => {},
  } = $props<{
    open?: boolean;
    skillName?: string;
    versionGroups?: SourceVersionGroup[];
    onConfirm?: (sourcePath: string) => Promise<void> | void;
    onCancel?: () => void;
  }>();

  let selectedSourcePath = $state("");

  $effect(() => {
    if (open) {
      selectedSourcePath = versionGroups[0]?.source_path ?? "";
    }
  });

  const closeModal = () => {
    open = false;
    onCancel();
  };

  const handleConfirm = async () => {
    if (!selectedSourcePath) return;
    await onConfirm(selectedSourcePath);
    open = false;
  };
</script>

<Modal bind:open title={$t("local.known.pathSelectTitle", { name: skillName })} onClose={closeModal}>
  <div class="flex h-full max-h-[90vh] w-full max-w-md flex-col">
    <div class="flex-1 overflow-y-auto px-6 pt-16 pb-6">
      <p class="text-base-content-muted mb-4 text-sm">{$t("local.sourceSelect.description")}</p>
      <SkillVersionGroupSelector
        groups={versionGroups}
        bind:selectedSourcePath
        radioName="knownVersionGroup"
      />
    </div>
    <div class="border-base-300 bg-base-100 flex items-center justify-end gap-2 border-t px-6 py-3 rounded-b-2xl">
      <button
        class="bg-primary text-primary-content hover:bg-primary-hover rounded-xl px-4 py-2 text-sm transition disabled:opacity-50"
        onclick={handleConfirm}
        disabled={!selectedSourcePath}
        type="button"
      >
        {$t("selectAgent.confirm")}
      </button>
    </div>
  </div>
</Modal>
