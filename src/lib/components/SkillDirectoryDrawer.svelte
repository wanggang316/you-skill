<script lang="ts">
  import { Loader2, X } from "@lucide/svelte";
  import IconButton from "./ui/IconButton.svelte";
  import { t } from "../i18n";
  import type { SkillDirectoryEntry } from "../api/skills";

  let {
    open = false,
    closing = false,
    loading = false,
    error = "",
    entries = [],
    activePath = "",
    onClose,
    onSelect,
  }: {
    open?: boolean;
    closing?: boolean;
    loading?: boolean;
    error?: string;
    entries?: SkillDirectoryEntry[];
    activePath?: string;
    onClose: () => void;
    onSelect: (path: string) => void;
  } = $props();

  const getEntryDepth = (entryPath: string) => entryPath.split("/").length - 1;
  const getEntryName = (entryPath: string) => entryPath.split("/").at(-1) || entryPath;

  const handleWindowKeydown = (event: KeyboardEvent) => {
    if (!open) return;
    if (event.key !== "Escape") return;
    event.preventDefault();
    onClose();
  };
</script>

<svelte:window onkeydown={handleWindowKeydown} />

{#if open}
  <button
    class="bg-overlay animate-drawer-backdrop fixed inset-0 z-[60] border-0 p-0"
    class:animate-drawer-backdrop-close={closing}
    onclick={onClose}
    type="button"
    aria-label={$t("detail.closeCatalog")}
  ></button>
  <aside
    class="border-base-300 bg-base-100 animate-drawer-panel fixed right-0 top-0 z-[65] h-full w-80 border-l shadow-2xl"
    class:animate-drawer-panel-close={closing}
  >
    <div class="border-base-300 flex items-center justify-between border-b px-4 py-3">
      <h2 class="text-base-content text-sm font-medium">{$t("detail.catalog")}</h2>
      <IconButton
        onclick={onClose}
        title={$t("detail.closeCatalog")}
        ariaLabel={$t("detail.closeCatalog")}
      >
        <X size={16} />
      </IconButton>
    </div>
    <div class="h-[calc(100%-57px)] overflow-y-auto p-2">
      {#if loading}
        <div class="text-base-content-muted flex items-center justify-center py-8 text-sm">
          <Loader2 size={18} class="animate-spin" />
        </div>
      {:else if error}
        <p class="text-error px-2 py-3 text-sm">{error}</p>
      {:else}
        {#each entries as entry (entry.path)}
          <button
            class="w-full rounded-lg px-2 py-1.5 text-left text-sm transition {entry.is_directory
              ? 'text-base-content-muted cursor-default'
              : entry.path === activePath
                ? 'bg-base-200 text-base-content'
                : 'text-base-content hover:bg-base-200'}"
            style={`padding-left:${getEntryDepth(entry.path) * 16 + 12}px;`}
            onclick={() => {
              if (entry.is_directory) return;
              onSelect(entry.path);
            }}
            disabled={entry.is_directory}
            type="button"
          >
            {getEntryName(entry.path)}
          </button>
        {/each}
      {/if}
    </div>
  </aside>
{/if}

<style>
  .animate-drawer-backdrop {
    animation: drawerBackdropIn 0.2s ease-out;
  }

  .animate-drawer-backdrop-close {
    animation: drawerBackdropOut 0.2s ease-out;
  }

  .animate-drawer-panel {
    animation: drawerSlideIn 0.2s ease-out;
  }

  .animate-drawer-panel-close {
    animation: drawerSlideOut 0.2s ease-out;
  }

  @keyframes drawerSlideIn {
    from {
      transform: translateX(100%);
    }
    to {
      transform: translateX(0);
    }
  }

  @keyframes drawerSlideOut {
    from {
      transform: translateX(0);
    }
    to {
      transform: translateX(100%);
    }
  }

  @keyframes drawerBackdropIn {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }

  @keyframes drawerBackdropOut {
    from {
      opacity: 1;
    }
    to {
      opacity: 0;
    }
  }
</style>
