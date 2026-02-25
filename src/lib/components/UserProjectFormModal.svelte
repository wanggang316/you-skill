<script lang="ts">
  import { confirm } from "@tauri-apps/plugin-dialog";
  import { Check, Folder, Loader2, Pencil, Plus, Trash2, X } from "@lucide/svelte";
  import Modal from "./ui/Modal.svelte";
  import { t } from "../i18n";
  import {
    addUserProject,
    listUserProjects,
    removeUserProject,
    updateUserProject,
    type UserProject,
  } from "../api/user-projects";

  interface Props {
    open?: boolean;
  }

  let { open = $bindable(false) }: Props = $props();

  let projects = $state<UserProject[]>([]);
  let loading = $state(false);
  let error = $state("");
  let editingOriginalName = $state("");
  let editName = $state("");
  let editPath = $state("");
  let saving = $state(false);
  let validationError = $state("");
  let addName = $state("");
  let addPath = $state("");
  let adding = $state(false);
  let addError = $state("");
  let showAddRow = $state(false);

  const modalTitle = $derived($t("projectManage.title"));
  const isEditing = $derived(editingOriginalName.length > 0);

  $effect(() => {
    if (!open) return;
    resetEdit();
    resetAdd();
    loadProjects();
  });

  function resetEdit() {
    editingOriginalName = "";
    editName = "";
    editPath = "";
    validationError = "";
  }

  function resetAdd() {
    addName = "";
    addPath = "";
    addError = "";
    showAddRow = false;
  }

  async function loadProjects() {
    loading = true;
    error = "";
    try {
      projects = await listUserProjects();
    } catch (err) {
      error = String(err);
    } finally {
      loading = false;
    }
  }

  function startEdit(project: UserProject) {
    editingOriginalName = project.name;
    editName = project.name;
    editPath = project.path;
    validationError = "";
    addError = "";
  }

  function cancelEdit() {
    resetEdit();
  }

  async function handleRemove(project: UserProject) {
    const confirmed = await confirm($t("projectManage.removeConfirm", { name: project.name }));
    if (!confirmed) return;

    try {
      await removeUserProject(project.name);
      await loadProjects();
    } catch (err) {
      error = String(err);
    }
  }

  async function pickFolderForEdit() {
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const result = await open({
        multiple: false,
        directory: true,
      });
      if (result) {
        editPath = result;
        validationError = "";
      }
    } catch (error) {
      console.error("Failed to select folder:", error);
    }
  }

  async function pickFolderForAdd() {
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const result = await open({
        multiple: false,
        directory: true,
      });
      if (result) {
        addPath = result;
        addError = "";
      }
    } catch (error) {
      console.error("Failed to select folder:", error);
    }
  }

  async function saveEdit() {
    if (!isEditing || !editName.trim() || !editPath.trim()) return;

    validationError = "";
    saving = true;
    try {
      await updateUserProject(editingOriginalName, editName.trim(), editPath.trim());
      await loadProjects();
      resetEdit();
    } catch (error) {
      validationError = String(error);
    } finally {
      saving = false;
    }
  }

  async function addProject() {
    if (!addName.trim() || !addPath.trim()) return;

    addError = "";
    adding = true;
    try {
      await addUserProject(addName.trim(), addPath.trim());
      await loadProjects();
      resetAdd();
    } catch (error) {
      addError = String(error);
    } finally {
      adding = false;
    }
  }

  function openAddRow() {
    showAddRow = true;
    addError = "";
  }

  function cancelAddRow() {
    showAddRow = false;
    addName = "";
    addPath = "";
    addError = "";
  }
</script>

<Modal bind:open title={modalTitle} containerClass="max-w-2xl">
  <div class="mx-auto min-h-[420px] w-full px-5 py-3">
    {#if loading}
      <div class="text-base-content-muted py-10 text-center text-sm">{$t("projectManage.loading")}</div>
    {:else}
      <div class="border-base-300 overflow-hidden rounded-xl border">
        <div
          class="bg-base-200 text-base-content-subtle grid grid-cols-[minmax(0,1fr)_minmax(0,2fr)_auto] gap-3 px-3 py-2 text-xs font-medium"
        >
          <div>{$t("projectManage.name")}</div>
          <div class="-ml-3">{$t("projectManage.path")}</div>
          <div></div>
        </div>

        {#if projects.length === 0}
          <div class="text-base-content-muted border-base-300 border-t px-3 py-6 text-center text-sm">
            {$t("projectManage.empty")}
          </div>
        {:else}
          {#each projects as project}
            <div
              class="border-base-300 grid grid-cols-[minmax(0,1fr)_minmax(0,2fr)_auto] items-center gap-3 border-t px-3 py-2"
            >
              {#if editingOriginalName === project.name}
                <input
                  type="text"
                  class="bg-base-100 text-base-content focus:ring-primary focus:border-primary border-base-300 w-full rounded-lg border px-3 py-1.5 text-sm focus:ring-2 focus:outline-none"
                  bind:value={editName}
                  oninput={() => {
                    validationError = "";
                  }}
                />
                <div class="flex items-center gap-2">
                  <input
                    type="text"
                    class="bg-base-100 text-base-content border-base-300 w-full rounded-lg border px-3 py-1.5 text-sm"
                    value={editPath}
                    readonly
                  />
                  <button
                    class="bg-base-100 text-base-content hover:bg-base-200 border-base-300 rounded-lg border px-2.5 py-1.5 transition"
                    onclick={pickFolderForEdit}
                    type="button"
                    title={$t("projectManage.selectFolder")}
                  >
                    <Folder size={16} />
                  </button>
                </div>
                <div class="flex justify-end gap-1">
                  <button
                    class="text-success hover:bg-success/10 rounded p-1.5"
                    onclick={saveEdit}
                    disabled={saving || !editName.trim() || !editPath.trim()}
                    type="button"
                    title={$t("projectManage.save")}
                  >
                    {#if saving}
                      <Loader2 size={12} class="animate-spin" />
                    {:else}
                      <Check size={12} />
                    {/if}
                  </button>
                  <button
                    class="text-base-content hover:bg-base-200 rounded p-1.5"
                    onclick={cancelEdit}
                    disabled={saving}
                    type="button"
                    title={$t("addSkill.cancel")}
                  >
                    <X size={12} />
                  </button>
                </div>
              {:else}
                <div class="text-base-content truncate text-sm font-medium">{project.name}</div>
                <div class="text-base-content-subtle truncate text-sm">{project.path}</div>
                <div class="flex justify-end gap-1">
                  <button
                    class="text-base-content hover:bg-base-200 rounded p-1.5"
                    onclick={() => startEdit(project)}
                    type="button"
                    title={$t("projectManage.edit")}
                  >
                    <Pencil size={12} />
                  </button>
                  <button
                    class="text-error hover:bg-error/10 rounded p-1.5"
                    onclick={() => handleRemove(project)}
                    type="button"
                    title={$t("projectManage.remove")}
                  >
                    <Trash2 size={12} />
                  </button>
                </div>
              {/if}
            </div>
          {/each}
        {/if}

        {#if showAddRow}
          <div
            class="border-base-300 bg-base-100 grid grid-cols-[minmax(0,1fr)_minmax(0,2fr)_auto] items-center gap-3 border-t px-3 py-2"
          >
            <input
              type="text"
              class="bg-base-100 text-base-content focus:ring-primary focus:border-primary border-base-300 w-full rounded-lg border px-3 py-1.5 text-sm focus:ring-2 focus:outline-none"
              placeholder={$t("projectManage.namePlaceholder")}
              bind:value={addName}
              oninput={() => {
                addError = "";
              }}
            />
            <div class="flex items-center gap-2">
              <input
                type="text"
                class="bg-base-100 text-base-content border-base-300 w-full rounded-lg border px-3 py-1.5 text-sm"
                placeholder={$t("projectManage.pathPlaceholder")}
                value={addPath}
                readonly
              />
              <button
                class="bg-base-100 text-base-content hover:bg-base-200 border-base-300 rounded-lg border px-2.5 py-1.5 transition"
                onclick={pickFolderForAdd}
                type="button"
                title={$t("projectManage.selectFolder")}
              >
                <Folder size={16} />
              </button>
            </div>
            <div class="flex justify-end gap-1">
              <button
                class="text-primary hover:bg-primary/10 rounded p-1.5"
                onclick={addProject}
                disabled={adding || !addName.trim() || !addPath.trim()}
                type="button"
                title={$t("projectManage.add")}
              >
                {#if adding}
                  <Loader2 size={12} class="animate-spin" />
                {:else}
                  <Check size={12} />
                {/if}
              </button>
              <button
                class="text-base-content hover:bg-base-200 rounded p-1.5"
                onclick={cancelAddRow}
                disabled={adding}
                type="button"
                title={$t("addSkill.cancel")}
              >
                <X size={12} />
              </button>
            </div>
          </div>
        {/if}
      </div>

      <div class="mt-3">
        <button
          class="text-primary hover:bg-primary/10 inline-flex items-center gap-1 rounded-lg px-3 py-2 text-sm transition disabled:opacity-50"
          onclick={openAddRow}
          disabled={showAddRow || adding}
          type="button"
        >
          <Plus size={14} />
          {$t("projectManage.add")}
        </button>
      </div>

      {#if error}
        <div class="text-error mt-3 text-sm">{error}</div>
      {/if}
      {#if validationError}
        <div class="text-error mt-2 text-sm">{validationError}</div>
      {/if}
      {#if addError}
        <div class="text-error mt-2 text-sm">{addError}</div>
      {/if}
    {/if}
  </div>
</Modal>
