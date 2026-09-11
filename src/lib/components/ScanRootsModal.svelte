<script lang="ts">
  import {
    AlertCircle,
    AlertTriangle,
    FolderOpen,
    FolderPlus,
    ScanSearch,
    X,
  } from "@lucide/svelte";
  import AgentBadge from "./AgentBadge.svelte";
  import Modal from "$lib/components/ui/Modal.svelte";
  import { t } from "../i18n";
  import { addScanRoot, removeScanRoot, suggestScanRoots } from "../api/scan-roots";
  import type { ScanRootSuggestion } from "../api/scan-roots";
  import { openInFileManager } from "../api/skills";
  import { agentsById } from "../stores/hub";
  import { homePath } from "../stores/env";
  import { closeScanRootsModal, openImportModal, scanRootsModal } from "../stores/modals";
  import { refreshScanRoots, scanRoots } from "../stores/scan-roots";

  let open = $state(false);
  let loading = $state(false);
  let busy = $state(false);
  let suggestions = $state<ScanRootSuggestion[]>([]);
  let error = $state("");

  $effect(() => {
    const next = $scanRootsModal;
    if (next.open && !open) void load();
    open = next.open;
  });

  const baseName = (value: string) => value.split(/[/\\]/).filter(Boolean).pop() || value;

  /** `~/.claude/skills` reads better than the absolute path in a narrow row. */
  const shortPath = (value: string) => {
    const home = $homePath;
    return home && value.startsWith(home) ? `~${value.slice(home.length)}` : value;
  };

  const formatDate = (value: string) => {
    const date = new Date(value);
    return Number.isNaN(date.getTime()) ? value : date.toLocaleString();
  };

  async function load() {
    loading = true;
    error = "";
    try {
      const [, found] = await Promise.all([refreshScanRoots(), suggestScanRoots()]);
      suggestions = found;
    } catch (err) {
      error = String(err);
    } finally {
      loading = false;
    }
  }

  async function register(name: string, path: string) {
    busy = true;
    error = "";
    try {
      await addScanRoot(name, path);
      await load();
    } catch (err) {
      error = String(err);
    } finally {
      busy = false;
    }
  }

  async function handleAddFolder() {
    try {
      const { open: openDialog } = await import("@tauri-apps/plugin-dialog");
      const result = await openDialog({ multiple: false, directory: true });
      if (typeof result !== "string") return;
      await register(baseName(result), result);
    } catch (err) {
      error = String(err);
    }
  }

  async function handleRemove(path: string) {
    busy = true;
    error = "";
    try {
      await removeScanRoot(path);
      await load();
    } catch (err) {
      error = String(err);
    } finally {
      busy = false;
    }
  }

  function handleScan(path: string) {
    handleClose();
    openImportModal({ tab: "folder", folder: path });
  }

  function handleOpenDir(path: string) {
    openInFileManager(path).catch((err) => (error = String(err)));
  }

  function handleClose() {
    open = false;
    closeScanRootsModal();
  }
</script>

<Modal bind:open title={$t("scanRoots.title")} onClose={handleClose} containerClass="max-w-2xl">
  <div class="flex h-[60vh] min-h-0 flex-col">
    <div
      class="border-base-200 flex flex-none items-start justify-between gap-3 border-b px-6 py-4"
    >
      <p class="text-base-content-muted text-xs">{$t("scanRoots.description")}</p>
      <button
        class="border-base-300 text-base-content hover:bg-base-200 flex h-8 shrink-0 items-center gap-1.5 rounded-xl border px-3 text-[13px] transition"
        type="button"
        onclick={handleAddFolder}
        disabled={busy}
      >
        <FolderPlus size={15} />
        {$t("scanRoots.add")}
      </button>
    </div>

    <div class="min-h-0 flex-1 overflow-y-auto px-3 py-2">
      {#if loading}
        <p class="text-base-content-faint px-3 py-10 text-center text-xs">
          {$t("scanRoots.loading")}
        </p>
      {:else if $scanRoots.length === 0}
        <p class="text-base-content-muted px-3 pt-8 pb-3 text-center text-xs">
          {$t("scanRoots.empty")}
        </p>
      {:else}
        {#each $scanRoots as root (root.path)}
          <div class="hover:bg-base-200 flex items-start gap-2.5 rounded-lg px-2.5 py-2 transition">
            <span class="min-w-0 flex-1">
              <span class="flex min-w-0 items-center gap-1.5">
                <span class="text-base-content truncate text-[13px] font-medium">{root.name}</span>
                {#if root.missing}
                  <span class="text-error shrink-0" title={$t("scanRoots.missing")}>
                    <AlertTriangle size={12} />
                  </span>
                {/if}
              </span>
              <span class="text-base-content-faint block truncate text-[11px]" title={root.path}>
                {shortPath(root.path)}
              </span>
              <span class="text-base-content-subtle block text-[11px]">
                {#if root.lastScannedAt}
                  {$t("scanRoots.lastScanned", {
                    time: formatDate(root.lastScannedAt),
                    count: root.lastFound ?? 0,
                  })}
                {:else}
                  {$t("scanRoots.never")}
                {/if}
              </span>
            </span>
            <span class="flex shrink-0 items-center gap-1">
              <button
                class="border-base-300 text-base-content hover:bg-base-100 flex h-7 items-center gap-1 rounded-lg border px-2 text-[12px] transition disabled:opacity-50"
                type="button"
                onclick={() => handleScan(root.path)}
                disabled={busy || root.missing}
              >
                <ScanSearch size={13} />
                {$t("scanRoots.scan")}
              </button>
              <button
                class="text-base-content-muted hover:bg-base-100 hover:text-base-content flex h-7 w-7 items-center justify-center rounded-lg transition disabled:opacity-50"
                type="button"
                onclick={() => handleOpenDir(root.path)}
                title={$t("detail.openDir")}
                aria-label={$t("detail.openDir")}
                disabled={busy || root.missing}
              >
                <FolderOpen size={14} />
              </button>
              <button
                class="text-base-content-muted hover:bg-base-100 hover:text-error flex h-7 w-7 items-center justify-center rounded-lg transition disabled:opacity-50"
                type="button"
                onclick={() => handleRemove(root.path)}
                title={$t("scanRoots.remove")}
                aria-label={$t("scanRoots.remove")}
                disabled={busy}
              >
                <X size={14} />
              </button>
            </span>
          </div>
        {/each}
      {/if}

      {#if !loading && suggestions.length > 0}
        <div class="border-base-200 mt-3 border-t pt-2">
          <p class="text-base-content-subtle px-3 pb-1.5 text-[11px] font-medium">
            {$t("scanRoots.suggestions")}
          </p>
          {#each suggestions as suggestion (suggestion.path)}
            <div class="flex items-center gap-2.5 rounded-lg px-2.5 py-2">
              <span
                class="text-base-content-muted min-w-0 flex-1 truncate text-[12px]"
                title={suggestion.path}
              >
                {shortPath(suggestion.path)}
              </span>
              <AgentBadge agentIds={suggestion.agentIds} agents={$agentsById} />
              <button
                class="border-base-300 text-base-content hover:bg-base-100 h-7 shrink-0 rounded-lg border px-2 text-[12px] transition disabled:opacity-50"
                type="button"
                onclick={() => register(baseName(suggestion.path), suggestion.path)}
                disabled={busy}
              >
                {$t("scanRoots.addSuggestion")}
              </button>
            </div>
          {/each}
        </div>
      {/if}

      {#if error}
        <div class="text-error flex items-start gap-2 px-3 py-2 text-sm whitespace-pre-wrap">
          <AlertCircle size={16} class="mt-0.5 shrink-0" />
          <span>{error}</span>
        </div>
      {/if}
    </div>
  </div>
  {#snippet footer()}
    <span class="text-base-content-faint mr-auto text-[11px]">{$t("scanRoots.rule")}</span>
    <button
      class="border-base-300 text-base-content hover:bg-base-200 rounded-xl border px-4 py-2 text-sm transition"
      type="button"
      onclick={handleClose}
    >
      {$t("common.close")}
    </button>
  {/snippet}
</Modal>
