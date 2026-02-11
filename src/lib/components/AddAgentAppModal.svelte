<script lang="ts">
  import { t } from "../i18n";
  import { addAgentApp, updateAgentApp, validateAgentApp, type AgentAppDetail } from "../api";
  import Modal from "./ui/Modal.svelte";
  import { Folder, Loader2 } from "@lucide/svelte";

  interface Props {
    open?: boolean;
    onAppsChange?: () => void;
    appToEdit?: AgentAppDetail | null;
  }

  let { open = $bindable(false), onAppsChange = () => {}, appToEdit = null }: Props = $props();

  // Form state
  let displayName = $state("");
  let globalPath = $state("");
  let projectPath = $state("");
  let adding = $state(false);
  let validating = $state(false);
  let validationErrors = $state<string[]>([]);
  let validationWarnings = $state<string[]>([]);

  // Reset form when modal opens or populate for editing
  $effect(() => {
    if (open) {
      if (appToEdit) {
        displayName = appToEdit.display_name;
        globalPath = appToEdit.global_path || "";
        projectPath = appToEdit.project_path || "";
      } else {
        resetForm();
      }
      validationErrors = [];
      validationWarnings = [];
    }
  });

  const isEditMode = $derived(appToEdit !== null);
  const modalTitle = $derived(isEditMode ? $t("agentApps.editTitle") : $t("agentApps.addTitle"));
  const buttonText = $derived(isEditMode ? $t("agentApps.save") : $t("agentApps.add"));

  function closeModal() {
    open = false;
  }

  function resetForm() {
    displayName = "";
    globalPath = "";
    projectPath = "";
    validationErrors = [];
    validationWarnings = [];
  }

  async function handleSelectFolder() {
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const result = await open({
        multiple: false,
        directory: true,
      });
      if (result) {
        globalPath = result;
        // Clear validation when path changes
        validationErrors = [];
        validationWarnings = [];
      }
    } catch (error) {
      console.error("Failed to select folder:", error);
    }
  }

  async function handleAdd() {
    if (!displayName.trim() || !globalPath.trim()) {
      return;
    }

    // Show validating state
    validating = true;
    validationErrors = [];
    validationWarnings = [];

    // Run validation (skip path duplicate check for edit mode)
    let validationResult: { errors: string[]; warnings: string[] } | null = null;
    try {
      validationResult = await validateAgentApp(displayName.trim(), globalPath.trim());
      // Filter out errors that are ok in edit mode:
      // - "already exists" for path/name if editing same app
      // - "already installed locally" is expected for existing apps
      if (isEditMode) {
        validationResult.errors = validationResult.errors.filter((err) => {
          // Keep error if it's about path/name duplication with different app
          if (err.includes("already exists")) {
            // Only filter out if the path is unchanged
            if (!appToEdit || globalPath.trim() !== appToEdit.global_path) {
              return true;
            }
            return false;
          }
          // Filter out "already installed locally" error in edit mode
          if (err.includes("already installed locally")) {
            return false;
          }
          return true;
        });
      }
      validationErrors = validationResult.errors;
      validationWarnings = validationResult.warnings;
    } catch (err) {
      validationErrors = [String(err)];
    } finally {
      validating = false;
    }

    // If validation passed, proceed to add or update
    if (validationErrors.length === 0) {
      adding = true;
      try {
        const projectPathValue = projectPath.trim() || undefined;
        if (isEditMode && appToEdit) {
          await updateAgentApp(
            appToEdit.id,
            displayName.trim(),
            globalPath.trim(),
            projectPathValue
          );
        } else {
          await addAgentApp(displayName.trim(), globalPath.trim(), projectPathValue);
        }
        closeModal();
        onAppsChange();
      } catch (err) {
        validationErrors = [String(err)];
      } finally {
        adding = false;
      }
    }
  }
</script>

<Modal bind:open title={modalTitle}>
  <div class="mx-auto flex h-[60vh] w-[50vw] max-w-lg flex-col">
    <!-- Form content -->
    <div class="flex-1 space-y-6 overflow-y-auto px-5 pt-14 pb-4">
      <div>
        <label class="text-base-content mb-1.5 block text-sm">
          {$t("agentApps.displayName")}
        </label>
        <input
          type="text"
          class="bg-base-100 text-base-content focus:ring-primary focus:border-primary border-base-300 w-full rounded-lg border px-4 py-2 text-sm focus:ring-2 focus:outline-none"
          placeholder={$t("agentApps.displayNamePlaceholder")}
          bind:value={displayName}
          oninput={() => {
            validationErrors = [];
            validationWarnings = [];
          }}
        />
      </div>

      <div>
        <label class="text-base-content mb-1.5 block text-sm">
          {$t("agentApps.globalPath")}
        </label>
        <div class="flex gap-2">
          <input
            type="text"
            class="bg-base-100 text-base-content focus:ring-primary focus:border-primary border-base-300 flex-1 rounded-lg border px-4 py-2 text-sm focus:ring-2 focus:outline-none"
            placeholder={$t("agentApps.globalPathPlaceholder")}
            bind:value={globalPath}
            oninput={() => {
              validationErrors = [];
              validationWarnings = [];
            }}
          />
          <button
            class="bg-base-100 text-base-content hover:bg-base-200 border-base-300 flex items-center justify-center rounded-lg px-3 transition"
            onclick={handleSelectFolder}
            type="button"
            title={$t("agentApps.selectFolder")}
          >
            <Folder size={20} />
          </button>
        </div>
      </div>

      <div>
        <label class="text-base-content mb-1.5 block text-sm">
          {$t("agentApps.projectPath")}
        </label>
        <input
          type="text"
          class="bg-base-100 text-base-content focus:ring-primary focus:border-primary border-base-300 w-full rounded-lg border px-4 py-2 text-sm focus:ring-2 focus:outline-none"
          placeholder={$t("agentApps.projectPathPlaceholder")}
          bind:value={projectPath}
        />
      </div>

      <!-- Validation messages -->
      {#if validationErrors.length > 0}
        <div class="space-y-1">
          {#each validationErrors as msg}
            <div class="text-error text-xs">{msg}</div>
          {/each}
        </div>
      {/if}

      {#if validationWarnings.length > 0}
        <div class="space-y-1">
          {#each validationWarnings as msg}
            <div class="text-warning text-xs">{msg}</div>
          {/each}
        </div>
      {/if}
    </div>

    <!-- Actions -->
    <div class="border-base-200 flex justify-end gap-3 border-t px-5 py-3">
      <button
        class="bg-primary text-primary-content hover:bg-primary-hover rounded-xl px-6 py-2 text-sm transition disabled:opacity-50"
        onclick={handleAdd}
        disabled={adding || validating || !displayName.trim() || !globalPath.trim()}
        type="button"
      >
        {#if validating}
          <Loader2 size={16} class="mr-1 inline animate-spin" />
        {:else if adding}
          <Loader2 size={16} class="mr-1 inline animate-spin" />
        {:else}
          {buttonText}
        {/if}
      </button>
    </div>
  </div>
</Modal>
