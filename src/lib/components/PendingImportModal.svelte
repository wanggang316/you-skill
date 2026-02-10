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

<Modal bind:open={isOpen} title={$t("pendingImport.title")} onClose={closeModal}>
  <div class="flex h-full max-h-[90vh] w-full max-w-lg flex-col">
    <!-- Content -->
    <div class="flex-1 overflow-y-auto p-6 pt-16">
      <!-- Description -->
      <div class="bg-base-200 mb-6 rounded-xl p-4">
        <p class="text-base-content text-sm">
          {$t("pendingImport.description", { count: unmanagedSkills.length })}
        </p>
      </div>

      <!-- Skills List -->
      {#if unmanagedSkills.length === 0}
        <div
          class="border-base-300 bg-base-100 text-base-content-muted rounded-xl border border-dashed p-8 text-center text-sm"
        >
          {$t("pendingImport.empty")}
        </div>
      {:else}
        <div class="space-y-3">
          {#each unmanagedSkills as skill}
            <div
              class="border-base-300 bg-base-100 hover:bg-base-200 rounded-2xl border p-4 transition hover:shadow-sm"
            >
              <div class="flex flex-wrap items-center justify-between gap-3">
                <div class="min-w-0 flex-1">
                  <p class="truncate text-base font-semibold">{skill.name}</p>
                  <div class="mt-2 flex flex-wrap gap-2">
                    {#each skill.agents as agentId}
                      <div
                        class="bg-base-200 text-base-content-subtle inline-flex items-center rounded-full px-2.5 py-1 text-xs"
                      >
                        {agentMap.get(agentId) || agentId}
                      </div>
                    {/each}
                  </div>
                  {#if skill.canonical_path}
                    <p
                      class="text-base-content-muted mt-2 truncate text-xs"
                      title={skill.canonical_path}
                    >
                      {skill.canonical_path}
                    </p>
                  {/if}
                </div>
                <div class="text-base-content-faint flex items-center gap-2 text-xs">
                  {#if skill.managed_status === "mixed"}
                    <span class="tag tag-warning">{$t("local.tag.standalone")}</span>
                  {/if}
                  {#if skill.name_conflict}
                    <span class="tag tag-error">{$t("local.tag.nameConflict")}</span>
                  {/if}
                  {#if skill.conflict_with_managed}
                    <span class="tag tag-neutral">
                      {$t("local.tag.conflictManaged")}
                    </span>
                  {/if}
                  <button
                    class="border-base-300 text-base-content-muted hover:bg-base-200 flex items-center rounded-lg border px-3 py-1.5 text-xs transition"
                    onclick={() => handleOpenFolder(skill)}
                    title={$t("pendingImport.openFolder")}
                    type="button"
                  >
                    <FolderOpen size={14} class="mr-1" />
                    {$t("pendingImport.open")}
                  </button>
                  <button
                    class="border-base-300 text-base-content-muted hover:bg-primary hover:text-primary-content hover:border-primary flex items-center rounded-lg border px-3 py-1.5 text-xs transition"
                    onclick={() => handleImport(skill)}
                    title={$t("local.action.import")}
                    type="button"
                  >
                    {$t("local.action.import")}
                  </button>
                  <button
                    class="border-error-border text-error hover:bg-error hover:text-error-content rounded-lg border px-3 py-1.5 text-xs transition"
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
      class="border-base-300 bg-base-100 flex justify-end gap-3 rounded-b-2xl border-t px-6 py-3"
    >
      <button
        class="bg-warning text-warning-content hover:bg-warning-hover flex items-center rounded-xl px-4 py-2 text-sm transition disabled:opacity-50"
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
