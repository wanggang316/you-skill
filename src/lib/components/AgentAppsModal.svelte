<script lang="ts">
  import { onMount } from "svelte";
  import { t } from "../i18n";
  import Modal from "./ui/Modal.svelte";
  import IconButton from "./ui/IconButton.svelte";
  import { Plus, Trash2, Check, ChevronRight } from "@lucide/svelte";
  import {
    listInternalAgentApps,
    listUserAgentApps,
    addAgentApp,
    removeAgentApp,
    validateAgentApp,
    listLocalAgentApps,
    type AgentAppDetail,
  } from "../api";

  interface Props {
    open?: boolean;
    onClose?: () => void;
    onAppsChange?: () => void;
  }

  let {
    open = $bindable(false),
    onClose = () => {},
    onAppsChange = () => {},
  }: Props = $props();

  // State
  let internalApps = $state<AgentAppDetail[]>([]);
  let userApps = $state<AgentAppDetail[]>([]);
  let localAppsIds = $state<Set<string>>(new Set());
  let loading = $state(false);
  let error = $state("");

  // Add form state
  let showAddForm = $state(false);
  let displayName = $state("");
  let globalPath = $state("");
  let projectPath = $state("");
  let validating = $state(false);
  let validationErrors = $state<string[]>([]);
  let validationWarnings = $state<string[]>([]);
  let adding = $state(false);

  // Load data when modal opens
  $effect(() => {
    if (open) {
      loadAgentApps();
    }
  });

  async function loadAgentApps() {
    loading = true;
    error = "";
    try {
      const [internal, user, local] = await Promise.all([
        listInternalAgentApps(),
        listUserAgentApps(),
        listLocalAgentApps(),
      ]);
      internalApps = internal;
      userApps = user;
      localAppsIds = new Set(local.map((app) => app.id));
    } catch (err) {
      error = String(err);
    } finally {
      loading = false;
    }
  }

  function handleClose() {
    showAddForm = false;
    resetForm();
    onClose();
  }

  function resetForm() {
    displayName = "";
    globalPath = "";
    projectPath = "";
    validationErrors = [];
    validationWarnings = [];
  }

  function openAddForm() {
    showAddForm = true;
    resetForm();
  }

  function closeAddForm() {
    showAddForm = false;
    resetForm();
  }

  async function handleDetect() {
    if (!displayName.trim() || !globalPath.trim()) {
      return;
    }

    validating = true;
    validationErrors = [];
    validationWarnings = [];
    try {
      const result = await validateAgentApp({
        display_name: displayName.trim(),
        global_path: globalPath.trim(),
      });
      validationErrors = result.errors;
      validationWarnings = result.warnings;
    } catch (err) {
      validationErrors = [String(err)];
    } finally {
      validating = false;
    }
  }

  async function handleAdd() {
    if (!displayName.trim() || !globalPath.trim()) {
      return;
    }

    if (validationErrors.length > 0) {
      return;
    }

    adding = true;
    try {
      const projectPathValue = projectPath.trim() || undefined;
      await addAgentApp({
        display_name: displayName.trim(),
        global_path: globalPath.trim(),
        project_path: projectPathValue,
      });
      // Reload data
      await loadAgentApps();
      closeAddForm();
      onAppsChange();
    } catch (err) {
      validationErrors = [String(err)];
    } finally {
      adding = false;
    }
  }

  async function handleRemove(id: string, name: string) {
    if (!confirm($t("agentApps.removeConfirm") + ` (${name})`)) {
      return;
    }

    try {
      await removeAgentApp(id);
      // Reload data
      await loadAgentApps();
      onAppsChange();
    } catch (err) {
      error = String(err);
    }
  }

  function isInstalled(id: string): boolean {
    return localAppsIds.has(id);
  }
</script>

<Modal bind:open title={$t("agentApps.modalTitle")} onClose={handleClose}>
  <div class="px-6 py-6 max-w-2xl">
    {#if loading}
      <div class="text-base-content-muted flex items-center justify-center py-12">
        <span class="loading loading-spinner loading-md"></span>
      </div>
    {:else if error}
      <div class="text-error py-8 text-center">{error}</div>
    {:else}
      <div class="space-y-6">
        <!-- Internal Agent Apps -->
        <section>
          <h4 class="text-base-content mb-3 text-sm font-medium">
            {$t("agentApps.internalSection")}
          </h4>
          <div class="bg-base-200 max-h-60 space-y-1 overflow-y-auto rounded-xl p-2">
            {#each internalApps as app}
              <div
                class="bg-base-100 flex items-center justify-between rounded-lg px-3 py-2.5"
              >
                <div class="flex items-center gap-3">
                  <span class="text-base-content text-sm">{app.display_name}</span>
                  {#if isInstalled(app.id)}
                    <span
                      class="text-success bg-success/10 text-xs rounded-full px-2 py-0.5 flex items-center gap-1"
                    >
                      <Check size={12} />
                      {$t("agentApps.installed")}
                    </span>
                  {:else}
                    <span
                      class="text-base-content-muted text-xs rounded-full px-2 py-0.5"
                    >
                      {$t("agentApps.notInstalled")}
                    </span>
                  {/if}
                </div>
                {#if app.global_path}
                  <span class="text-base-content-muted text-xs">{app.global_path}</span>
                {/if}
              </div>
            {/each}
          </div>
        </section>

        <!-- User Agent Apps -->
        <section>
          <div class="mb-3 flex items-center justify-between">
            <h4 class="text-base-content text-sm font-medium">
              {$t("agentApps.userSection")}
            </h4>
          </div>
          <div class="bg-base-200 max-h-60 space-y-1 overflow-y-auto rounded-xl p-2">
            {#if userApps.length === 0}
              <div class="text-base-content-muted py-4 text-center text-sm">
                No custom agent apps yet
              </div>
            {:else}
              {#each userApps as app}
                <div
                  class="bg-base-100 flex items-center justify-between rounded-lg px-3 py-2.5"
                >
                  <div class="flex items-center gap-3">
                    <span class="text-base-content text-sm">{app.display_name}</span>
                    {#if isInstalled(app.id)}
                      <span
                        class="text-success bg-success/10 text-xs rounded-full px-2 py-0.5 flex items-center gap-1"
                      >
                        <Check size={12} />
                        {$t("agentApps.installed")}
                      </span>
                    {:else}
                      <span
                        class="text-base-content-muted text-xs rounded-full px-2 py-0.5"
                      >
                        {$t("agentApps.notInstalled")}
                      </span>
                    {/if}
                  </div>
                  <div class="flex items-center gap-2">
                    {#if app.global_path}
                      <span class="text-base-content-muted text-xs">{app.global_path}</span>
                    {/if}
                    <button
                      class="text-error hover:bg-error/10 rounded p-1"
                      onclick={() => handleRemove(app.id, app.display_name)}
                      type="button"
                      title={$t("agentApps.remove")}
                    >
                      <Trash2 size={14} />
                    </button>
                  </div>
                </div>
              {/each}
            {/if}
          </div>
        </section>

        <!-- Missing my app section -->
        <section class="text-base-content-muted flex items-center justify-between text-sm">
          <span>{$t("agentApps.missingMyApp")}</span>
          <button
            class="text-primary hover:bg-primary/10 flex items-center gap-1 rounded-lg px-3 py-1.5 text-sm font-medium"
            onclick={openAddForm}
            type="button"
          >
            <Plus size={14} />
            {$t("agentApps.manualAdd")}
          </button>
        </section>
      </div>
    {/if}

    <!-- Add Form Modal (overlay) -->
    {#if showAddForm}
      <div
        class="bg-base-100 border-base-200 absolute inset-0 top-0 left-0 z-10 rounded-2xl border p-6"
      >
        <h3 class="text-base-content mb-4 text-base font-medium">
          {$t("agentApps.addTitle")}
        </h3>

        <div class="space-y-4">
          <div>
            <label class="text-base-content mb-1.5 block text-sm font-medium">
              {$t("agentApps.displayName")}
            </label>
            <input
              type="text"
              class="bg-base-200 text-base-content w-full rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary"
              placeholder={$t("agentApps.displayNamePlaceholder")}
              bind:value={displayName}
              oninput={() => {
                validationErrors = [];
                validationWarnings = [];
              }}
            />
          </div>

          <div>
            <label class="text-base-content mb-1.5 block text-sm font-medium">
              {$t("agentApps.globalPath")}
            </label>
            <input
              type="text"
              class="bg-base-200 text-base-content w-full rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary"
              placeholder={$t("agentApps.globalPathPlaceholder")}
              bind:value={globalPath}
              oninput={() => {
                validationErrors = [];
                validationWarnings = [];
              }}
            />
          </div>

          <div>
            <label class="text-base-content mb-1.5 block text-sm font-medium">
              {$t("agentApps.projectPath")}
            </label>
            <input
              type="text"
              class="bg-base-200 text-base-content w-full rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary"
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

          <!-- Actions -->
          <div class="flex justify-end gap-2">
            <button
              class="text-base-content hover:bg-base-300 rounded-lg px-4 py-2 text-sm"
              onclick={closeAddForm}
              disabled={adding}
              type="button"
            >
              {$t("agentApps.cancel")}
            </button>
            <button
              class="bg-primary text-primary-content hover:bg-primary-hover rounded-lg px-4 py-2 text-sm disabled:opacity-50"
              onclick={handleDetect}
              disabled={validating || adding || !displayName.trim() || !globalPath.trim()}
              type="button"
            >
              {#if validating}
                <span class="loading loading-spinner loading-xs"></span>
              {/if}
              {$t("agentApps.detect")}
            </button>
            <button
              class="bg-success text-success-content hover:bg-success-hover rounded-lg px-4 py-2 text-sm disabled:opacity-50"
              onclick={handleAdd}
              disabled={adding || validationErrors.length > 0}
              type="button"
            >
              {#if adding}
                <span class="loading loading-spinner loading-xs"></span>
              {/if}
              {$t("agentApps.add")}
            </button>
          </div>
        </div>
      </div>
    {/if}
  </div>
</Modal>
