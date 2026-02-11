<script lang="ts">
  import { onMount } from "svelte";
  import { confirm } from "@tauri-apps/plugin-dialog";
  import { goto } from "$app/navigation";
  import {
    listInternalAgentApps,
    listUserAgentApps,
    removeAgentApp,
    refreshAgentApps,
    type AgentAppDetail,
  } from "../../lib/api";
  import { t } from "../../lib/i18n";
  import { Plus, Trash2, Check, Loader2, Pencil, ChevronLeft, RefreshCw } from "@lucide/svelte";
  import AddAgentAppModal from "../../lib/components/AddAgentAppModal.svelte";
  import {
    listAgents,
    type AgentInfo,
  } from "../../lib/api/skills";

  // State
  let internalApps = $state<AgentAppDetail[]>([]);
  let userApps = $state<AgentAppDetail[]>([]);
  let localAppsIds = $state<Set<string>>(new Set());
  let loading = $state(false);
  let error = $state("");
  let showAddModal = $state(false);
  let editingApp = $state<AgentAppDetail | null>(null);

  let agents = $state<AgentInfo[]>([]);

  onMount(() => {
    loadAgentApps();
    loadAgents().then((agentList) => {
      agents = agentList;
    });
  });

  async function loadAgents() {
    try {
      const agentList = await listAgents();
      return agentList;
    } catch (err) {
      console.error("Failed to load agents:", err);
      return [];
    }
  }

  async function loadAgentApps() {
    loading = true;
    error = "";
    try {
      const [internal, user, local] = await Promise.all([
        listInternalAgentApps(),
        listUserAgentApps(),
        refreshAgentApps(),
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
      await loadAgentApps();
    } catch (err) {
      error = String(err);
    }
  }

  function isInstalled(id: string): boolean {
    return localAppsIds.has(id);
  }

  function goBack() {
    goto("/");
  }
</script>

<div class="bg-base-100 text-base-content flex h-screen flex-col overflow-hidden">
  <!-- Header -->
  <header class="border-base-300 bg-base-100 sticky top-0 z-50 border-b">
    <div class="mx-auto flex max-w-6xl items-center justify-between px-6 py-4">
      <div class="flex w-full items-center justify-between">
        <div class="flex items-center gap-4">
          <button
            class="border-base-300 text-base-content hover:bg-base-200 flex items-center rounded-xl border p-2 text-sm transition"
            onclick={goBack}
            title={$t("header.back")}
            type="button"
          >
            <ChevronLeft size={16} />
          </button>
          <h1 class="text-base-content text-lg font-medium">
            {$t("agentApps.title")}
          </h1>
        </div>
        <button
          class="border-base-300 text-base-content hover:bg-base-200 flex items-center gap-2 rounded-xl border px-3 py-2 text-sm transition disabled:opacity-50"
          onclick={loadAgentApps}
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
    </div>
  </header>

  <main class="flex-1 overflow-y-auto">
    <section class="mx-auto max-w-4xl px-6 py-6">
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
                        >{app.global_path}</span>
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
  </main>
</div>

<!-- Add Agent App Modal -->
<AddAgentAppModal
  bind:open={showAddModal}
  appToEdit={editingApp}
  onAppsChange={loadAgentApps}
/>
