<script>
  import { FolderOpen, Trash2, Download } from "@lucide/svelte";
  import { t } from "../i18n";
  import { open } from "@tauri-apps/plugin-shell";
  import Modal from "./ui/Modal.svelte";

  let {
    open: isOpen = $bindable(false),
    unmanagedSkills = [],
    agentMap = new Map(),
    onImport = () => {},
    onImportAll = () => {},
    onDelete = () => {},
  } = $props();

  function closeModal() {
    isOpen = false;
  }

  async function handleOpenFolder(skill) {
    try {
      if (skill.canonical_path) {
        await open(skill.canonical_path);
      }
    } catch (error) {
      console.error("Failed to open folder:", error);
    }
  }

  function handleImport(skill) {
    onImport(skill);
  }

  function handleImportAll() {
    onImportAll();
    closeModal();
  }

  function handleDelete(skill) {
    onDelete(skill);
  }
</script>

<Modal
  bind:open={isOpen}
  title={$t("pendingImport.title")}
  onClose={closeModal}
>
  <div class="flex h-full max-h-[90vh] w-full max-w-lg flex-col">
    <!-- Content -->
    <div class="flex-1 overflow-y-auto p-6 pt-16">
      <!-- Description -->
      <div class="mb-6 rounded-xl bg-base-200 p-4">
        <p class="text-sm text-base-content">
          {$t("pendingImport.description", { count: unmanagedSkills.length })}
        </p>
      </div>

      <!-- Skills List -->
      {#if unmanagedSkills.length === 0}
        <div
          class="rounded-xl border border-dashed border-base-300 bg-base-100 p-8 text-center text-sm text-base-content-muted"
        >
          {$t("pendingImport.empty")}
        </div>
      {:else}
        <div class="space-y-3">
          {#each unmanagedSkills as skill}
            <div
              class="rounded-2xl border border-base-300 bg-base-100 p-4 transition hover:bg-base-200 hover:shadow-sm"
            >
              <div class="flex flex-wrap items-center justify-between gap-3">
                <div class="flex-1 min-w-0">
                  <p class="text-base font-semibold truncate">{skill.name}</p>
                  <div class="mt-2 flex flex-wrap gap-2">
                    {#each skill.agents as agentId}
                      <div
                        class="inline-flex items-center rounded-full bg-base-200 px-2.5 py-1 text-xs text-base-content-subtle"
                      >
                        {agentMap.get(agentId) || agentId}
                      </div>
                    {/each}
                  </div>
                  {#if skill.canonical_path}
                    <p
                      class="mt-2 text-xs text-base-content-muted truncate"
                      title={skill.canonical_path}
                    >
                      {skill.canonical_path}
                    </p>
                  {/if}
                </div>
                <div
                  class="flex items-center gap-2 text-xs text-base-content-faint"
                >
                  {#if skill.managed_status === "mixed"}
                    <span class="tag tag-warning"
                      >{$t("local.tag.standalone")}</span
                    >
                  {/if}
                  {#if skill.name_conflict}
                    <span class="tag tag-error"
                      >{$t("local.tag.nameConflict")}</span
                    >
                  {/if}
                  {#if skill.conflict_with_managed}
                    <span class="tag tag-neutral">
                      {$t("local.tag.conflictManaged")}
                    </span>
                  {/if}
                  <button
                    class="rounded-lg border border-base-300 px-3 py-1.5 text-xs text-base-content-muted transition hover:bg-base-200 flex items-center"
                    onclick={() => handleOpenFolder(skill)}
                    title={$t("pendingImport.openFolder")}
                    type="button"
                  >
                    <FolderOpen size={14} class="mr-1" />
                    {$t("pendingImport.open")}
                  </button>
                  <button
                    class="rounded-lg border border-base-300 px-3 py-1.5 text-xs text-base-content-muted transition hover:bg-primary hover:text-primary-content hover:border-primary flex items-center"
                    onclick={() => handleImport(skill)}
                    title={$t("local.action.import")}
                    type="button"
                  >
                    {$t("local.action.import")}
                  </button>
                  <button
                    class="rounded-lg border border-error-border px-3 py-1.5 text-xs text-error transition hover:bg-error hover:text-error-content"
                    type="button"
                    onclick={() => handleDelete(skill)}
                    title={$t("local.action.delete")}
                  >
                    <Trash2 size={14} />
                  </button>
                </div>
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </div>

    <!-- Footer -->
    <div
      class="flex justify-end gap-3 border-t border-base-300 px-6 py-4 bg-base-100 rounded-b-2xl"
    >
      <button
        class="rounded-xl bg-warning px-4 py-2 text-sm text-warning-content transition hover:bg-warning-hover disabled:opacity-50 flex items-center"
        onclick={handleImportAll}
        disabled={unmanagedSkills.length === 0}
        type="button"
      >
        <Download size={16} class="mr-2" />
        {$t("local.action.importAll")}
      </button>
    </div>
  </div>
</Modal>
