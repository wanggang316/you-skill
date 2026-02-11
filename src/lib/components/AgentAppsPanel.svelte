<script lang="ts">
  import { onMount } from "svelte";
  import { t } from "../i18n";
  import { confirm } from "@tauri-apps/plugin-dialog";
  import {
    listInternalAgentApps,
    listUserAgentApps,
    removeAgentApp,
    refreshAgentApps,
    type AgentAppDetail,
  } from "../api";
  import { Plus, Trash2, Check, Loader2, Pencil } from "@lucide/svelte";
  import AddAgentAppModal from "./AddAgentAppModal.svelte";

  interface Props {
    onBack?: () => void;
    onAppsChange?: () => void;
    onLoadingChange?: (loading: boolean) => void;
    exposeRefresh?: (refresh: () => void) => void;
  }

  let {
    onBack = () => {},
    onAppsChange = () => {},
    onLoadingChange,
    exposeRefresh,
  }: Props = $props();

  // Handle apps changed from modal
  function handleAppsChanged() {
    loadAgentApps();
    onAppsChange();
  }

  // Expose refresh function to parent
  $effect(() => {
    exposeRefresh?.(refreshData);
  });

  // State
  let internalApps = $state<AgentAppDetail[]>([]);
  let userApps = $state<AgentAppDetail[]>([]);
  let localAppsIds = $state<Set<string>>(new Set());
  let loading = $state(false);
  let error = $state("");
  let showAddModal = $state(false);
  let editingApp = $state<AgentAppDetail | null>(null);

  // Load data on mount
  onMount(() => {
    loadAgentApps();
  });

  async function loadAgentApps() {
    loading = true;
    onLoadingChange?.(true);
    error = "";
    try {
      const [internal, user, local] = await Promise.all([
        listInternalAgentApps(),
        listUserAgentApps(),
        refreshAgentApps(), // Clear cache and re-scan filesystem
      ]);
      internalApps = internal;
      userApps = user;
      localAppsIds = new Set(local.map((app) => app.id));
    } catch (err) {
      error = String(err);
    } finally {
      loading = false;
      onLoadingChange?.(false);
    }
  }

  function openAddModal() {
    editingApp = null;
    showAddModal = true;
  }

  function openEditModal(app: AgentAppDetail) {
    editingApp = app;
    showAddModal = true;
  }

  async function handleRemove(id: string, name: string) {
    const confirmed = await confirm($t("agentApps.removeConfirm") + ` (${name})`);
    if (!confirmed) {
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

  // Manual refresh - same as loadAgentApps since we always want fresh data
  async function refreshData() {
    await loadAgentApps();
  }
</script>

<section class="mx-auto max-w-4xl">
  {#if loading}
    <div class="text-base-content-muted flex items-center justify-center py-20">
      <Loader2 size={32} class="animate-spin" />
    </div>
  {:else if error}
    <div class="text-error py-8 text-center">{error}</div>
  {:else}
    <!-- Agent Apps List -->
    <div class="space-y-6">
      <!-- Internal Agent Apps (only show installed) -->
      {#if internalApps.some((app) => isInstalled(app.id))}
        <div class="bg-base-100">
          <div class="grid grid-cols-2 gap-2">
            {#each internalApps.filter((app) => isInstalled(app.id)) as app}
              <div class="bg-base-200 hover:bg-base-300 flex flex-col rounded-xl px-3 py-2">
                <span class="text-base-content text-xs font-medium">{app.display_name}</span>
                {#if app.global_path}
                  <span class="text-base-content/50 mt-0.5 truncate text-[10px]"
                    >{app.global_path}</span
                  >
                {/if}
              </div>
            {/each}
          </div>
        </div>
      {/if}

      <!-- User Agent Apps (only show if has apps) -->
      {#if userApps.length > 0}
        <div class="bg-base-100">
          <div class="mb-3 flex items-center justify-between">
            <h3 class="text-base-content text-base font-medium">
              {$t("agentApps.userSection")}
            </h3>
          </div>
          <div class="grid grid-cols-2 gap-2">
            {#each userApps as app}
              <div
                class="bg-base-200 hover:bg-base-300 flex flex-col justify-between gap-2 rounded-xl px-3 py-2"
              >
                <div class="flex items-start justify-between gap-2">
                  <span class="text-base-content text-xs font-medium">{app.display_name}</span>
                  <div class="flex gap-1">
                    <button
                      class="text-base-content hover:bg-base-200 shrink-0 rounded p-0.5"
                      onclick={() => openEditModal(app)}
                      type="button"
                      title={$t("agentApps.edit")}
                    >
                      <Pencil size={10} />
                    </button>
                    <button
                      class="text-error hover:bg-error/10 shrink-0 rounded p-0.5"
                      onclick={() => handleRemove(app.id, app.display_name)}
                      type="button"
                      title={$t("agentApps.remove")}
                    >
                      <Trash2 size={10} />
                    </button>
                  </div>
                </div>
                <div class="flex items-center justify-between gap-2">
                  {#if app.global_path}
                    <span class="text-base-content/50 truncate text-[10px]">{app.global_path}</span>
                  {/if}
                  {#if isInstalled(app.id)}
                    <span
                      class="text-success bg-success/10 flex shrink-0 items-center gap-1 rounded-full px-2 py-0.5 text-[10px] font-medium"
                    >
                      <Check size={10} />
                      {$t("agentApps.installed")}
                    </span>
                  {:else}
                    <span
                      class="text-base-content-muted shrink-0 rounded-full px-2 py-0.5 text-[10px]"
                    >
                      {$t("agentApps.notInstalled")}
                    </span>
                  {/if}
                </div>
              </div>
            {/each}
          </div>
        </div>
      {/if}

      <!-- Add new app section -->
      <div class="bg-base-100">
        <div class="text-base-content-muted flex items-center gap-2 text-xs">
          <span>{$t("agentApps.missingMyApp")}</span>
          <button
            class="text-primary hover:bg-primary/10 flex items-center gap-0.5 rounded-lg px-2 py-1 text-xs"
            onclick={openAddModal}
            type="button"
          >
            <Plus size={16} />
            {$t("agentApps.add")}
          </button>
        </div>
      </div>
    </div>
  {/if}
</section>

<!-- Add Agent App Modal -->
<AddAgentAppModal
  bind:open={showAddModal}
  appToEdit={editingApp}
  onAppsChange={handleAppsChanged}
/>
