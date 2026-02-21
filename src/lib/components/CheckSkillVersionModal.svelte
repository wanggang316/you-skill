<script lang="ts">
  import Modal from "./ui/Modal.svelte";
  import PrimaryActionButton from "./ui/PrimaryActionButton.svelte";
  import { t } from "../i18n";
  import type { SourceVersionGroup } from "../api/skills";

  let {
    open = $bindable(false),
    title = "",
    skillName = "",
    versionGroups = [],
    onConfirm = () => {},
    onCancel = () => {},
  } = $props<{
    open?: boolean;
    title?: string;
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

<Modal
  bind:open
  title={title || $t("local.sourceSelect.title", { name: skillName })}
  onClose={closeModal}
  containerClass="max-w-lg"
>
  <div class="w-full px-6 pt-4 pb-6">
    <p class="text-base-content-muted mb-4 text-sm">{$t("local.sourceSelect.description")}</p>
    <div class="space-y-3">
      {#each versionGroups as group}
        <label
          class="border-base-300 bg-base-100 hover:bg-base-200 flex w-full cursor-pointer items-start gap-3 rounded-lg border px-3 py-2"
        >
          <input
            class="mt-1 shrink-0"
            type="radio"
            name="skillVersionGroup"
            value={group.source_path}
            checked={selectedSourcePath === group.source_path}
            onchange={() => {
              selectedSourcePath = group.source_path;
            }}
          />
          <div class="min-w-0 flex-1">
            <div class="text-base-content mb-1 text-sm font-medium">{group.version}</div>
            <div class="space-y-1">
              {#each group.paths as path}
                <div class="text-base-content-subtle text-xs break-all">{path}</div>
              {/each}
            </div>
          </div>
        </label>
      {/each}
    </div>
  </div>
  {#snippet footer()}
    <button
      class="border-base-300 text-base-content hover:bg-base-200 rounded-xl border px-4 py-2 text-sm transition"
      onclick={closeModal}
      type="button"
    >
      {$t("addSkill.cancel")}
    </button>
    <PrimaryActionButton onclick={handleConfirm} disabled={!selectedSourcePath}>
      {$t("selectAgent.confirm")}
    </PrimaryActionButton>
  {/snippet}
</Modal>
