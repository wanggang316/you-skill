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
  import { Plus, Trash2, Check, Loader2, RefreshCw } from "@lucide/svelte";
  import AddAgentAppModal from "./AddAgentAppModal.svelte";

  interface Props {
    onBack?: () => void;
    onAppsChange?: () => void;
  }

  let { onBack = () => {}, onAppsChange = () => {} }: Props = $props();

  // State
  let internalApps = $state<AgentAppDetail[]>([]);
  let userApps = $state<AgentAppDetail[]>([]);
  let localAppsIds = $state<Set<string>>(new Set());
  let loading = $state(false);
  let error = $state("");
  let showAddModal = $state(false);

  // Load data on mount
  onMount(() => {
    loadAgentApps();
  });

  async function loadAgentApps() {
    loading = true;
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
    }
  }

  function openAddModal() {
    showAddModal = true;
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

  // Manual refresh - same as loadAgentApps since we always want fresh data
  async function refreshData() {
    await loadAgentApps();
  }
</script>

<section class="mx-auto max-w-4xl">
  <div class="mb-6 flex items-center justify-between">
    <div>
      <h2 class="text-base-content text-2xl font-semibold">{$t("agentApps.title")}</h2>
      <p class="text-base-content-muted mt-1 text-sm">
        {$t("agentApps.subtitle")}
      </p>
    </div>
    <button
      class="text-base-content hover:bg-base-200 flex items-center gap-2 rounded-lg px-3 py-2 text-sm font-medium disabled:opacity-50"
      onclick={refreshData}
      disabled={loading}
      type="button"
    >
      {#if loading}
        <Loader2 size={16} class="animate-spin" />
      {:else}
        <RefreshCw size={16} />
      {/if}
      {$t("local.refresh")}
    </button>
  </div>

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
        <div class="bg-base-200 rounded-2xl p-4">
          <h3 class="text-base-content mb-3 text-base font-medium">
            {$t("agentApps.internalSection")}
          </h3>
          <div class="space-y-2">
            {#each internalApps.filter((app) => isInstalled(app.id)) as app}
              <div class="bg-base-100 flex items-center justify-between rounded-xl px-4 py-3">
                <div class="flex items-center gap-3">
                  <span class="text-base-content text-sm font-medium">{app.display_name}</span>
                  {#if isInstalled(app.id)}
                    <span
                      class="text-success bg-success/10 flex items-center gap-1.5 rounded-full px-2.5 py-1 text-xs font-medium"
                    >
                      <Check size={12} />
                      {$t("agentApps.installed")}
                    </span>
                  {:else}
                    <span class="text-base-content-muted rounded-full px-2.5 py-1 text-xs">
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
        </div>
      {/if}

      <!-- User Agent Apps (only show if has apps) -->
      {#if userApps.length > 0}
        <div class="bg-base-200 rounded-2xl p-4">
          <div class="mb-3 flex items-center justify-between">
            <h3 class="text-base-content text-base font-medium">
              {$t("agentApps.userSection")}
            </h3>
          </div>
          <div class="space-y-2">
            {#each userApps as app}
              <div class="bg-base-100 flex items-center justify-between rounded-xl px-4 py-3">
                <div class="flex items-center gap-3">
                  <span class="text-base-content text-sm font-medium">{app.display_name}</span>
                  {#if isInstalled(app.id)}
                    <span
                      class="text-success bg-success/10 flex items-center gap-1.5 rounded-full px-2.5 py-1 text-xs font-medium"
                    >
                      <Check size={12} />
                      {$t("agentApps.installed")}
                    </span>
                  {:else}
                    <span class="text-base-content-muted rounded-full px-2.5 py-1 text-xs">
                      {$t("agentApps.notInstalled")}
                    </span>
                  {/if}
                </div>
                <div class="flex items-center gap-3">
                  {#if app.global_path}
                    <span class="text-base-content-muted text-xs">{app.global_path}</span>
                  {/if}
                  <button
                    class="text-error hover:bg-error/10 rounded p-1.5"
                    onclick={() => handleRemove(app.id, app.display_name)}
                    type="button"
                    title={$t("agentApps.remove")}
                  >
                    <Trash2 size={14} />
                  </button>
                </div>
              </div>
            {/each}
          </div>
        </div>
      {/if}

      <!-- Add new app section -->
      <div class="bg-base-200 rounded-2xl p-4">
        <div class="text-base-content-muted flex items-center justify-between text-sm">
          <span>{$t("agentApps.missingMyApp")}</span>
          <button
            class="text-primary hover:bg-primary/10 flex items-center gap-1.5 rounded-lg px-3 py-2 text-sm font-medium"
            onclick={openAddModal}
            type="button"
          >
            <Plus size={16} />
            {$t("agentApps.manualAdd")}
          </button>
        </div>
      </div>
    </div>
  {/if}
</section>

<!-- Add Agent App Modal -->
<AddAgentAppModal bind:open={showAddModal} onAppsChange={onAppsChange} />
