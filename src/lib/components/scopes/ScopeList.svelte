<script lang="ts">
  import { Folder, FolderOpen, FolderPlus, Loader2, RefreshCw, UserRound } from "@lucide/svelte";
  import IconButton from "$lib/components/ui/IconButton.svelte";
  import { t } from "$lib/i18n";
  import type { ScopeEntry } from "$lib/scopes";

  let {
    entries = [],
    selectedKey = null,
    loading = false,
    error = "",
    onSelect,
    onAddProject,
    onRefresh,
  }: {
    entries?: ScopeEntry[];
    selectedKey?: string | null;
    loading?: boolean;
    error?: string;
    onSelect: (entry: ScopeEntry) => void;
    onAddProject: () => void;
    onRefresh: () => void;
  } = $props();

  const userEntries = $derived(entries.filter((entry) => entry.kind === "user"));
  const projectEntries = $derived(entries.filter((entry) => entry.kind === "project"));
</script>

{#snippet row(entry: ScopeEntry)}
  {@const selected = entry.key === selectedKey}
  <button
    class={`mx-1.5 mb-0.5 flex w-[calc(100%-0.75rem)] items-start gap-2.5 rounded-xl px-2.5 py-2 text-left transition ${
      selected ? "bg-base-300" : "hover:bg-base-200"
    }`}
    type="button"
    onclick={() => onSelect(entry)}
    aria-current={selected ? "true" : undefined}
  >
    <span class="text-base-content-subtle mt-0.5 shrink-0">
      {#if entry.kind === "user"}
        <UserRound size={15} />
      {:else if selected}
        <FolderOpen size={15} />
      {:else}
        <Folder size={15} />
      {/if}
    </span>
    <span class="min-w-0 flex-1">
      <span class="flex min-w-0 items-center gap-1.5">
        <span class="text-base-content truncate text-[13px] font-medium">{entry.name}</span>
        {#if entry.unregistered}
          <span class="tag tag-neutral shrink-0">{$t("projects.unregistered")}</span>
        {/if}
      </span>
      <span
        class="text-base-content-faint block truncate text-[11px]"
        title={entry.path || undefined}
      >
        {entry.path || $t("projects.userHint")}
      </span>
    </span>
    {#if entry.driftCount > 0}
      <span class="tag tag-warning mt-0.5 shrink-0">{entry.driftCount}</span>
    {:else if entry.skills.length > 0}
      <span class="text-base-content-faint mt-0.5 shrink-0 text-[11px]">{entry.skills.length}</span>
    {/if}
  </button>
{/snippet}

<div class="border-base-300 flex min-h-0 min-w-0 flex-col border-r">
  <header
    class="border-base-300 flex h-12 flex-none items-center justify-between border-b px-4"
    data-window-drag-region
  >
    <h1 class="text-base-content truncate text-[1.05rem] font-semibold tracking-[-0.02em]">
      {$t("projects.title")}
    </h1>
    <IconButton
      variant="outline"
      onclick={onRefresh}
      title={$t("library.refresh")}
      ariaLabel={$t("library.refresh")}
      class="h-8 w-8 p-0"
    >
      <RefreshCw size={15} class={loading ? "animate-spin" : ""} />
    </IconButton>
  </header>

  <div
    class="min-h-0 flex-1 overflow-y-auto py-2 [scrollbar-color:var(--scrollbar-thumb)_transparent] [scrollbar-width:thin]"
  >
    {#if error}
      <p class="text-error px-4 py-3 text-xs">{error}</p>
    {/if}

    <p class="text-base-content-subtle px-4 pb-1.5 text-[11px] font-medium tracking-wide uppercase">
      {$t("projects.userScope")}
    </p>
    {#each userEntries as entry (entry.key)}
      {@render row(entry)}
    {/each}

    <div class="flex items-center justify-between px-4 pt-3 pb-1.5">
      <p class="text-base-content-subtle text-[11px] font-medium tracking-wide uppercase">
        {$t("sidebar.projects")}
      </p>
      <button
        class="text-base-content-subtle hover:bg-base-300 hover:text-base-content inline-flex size-6 items-center justify-center rounded-md transition"
        type="button"
        onclick={onAddProject}
        title={$t("projectManage.title")}
        aria-label={$t("projectManage.title")}
      >
        <FolderPlus size={14} />
      </button>
    </div>
    {#if loading && projectEntries.length === 0}
      <div class="text-base-content-muted flex items-center justify-center gap-2 py-6 text-xs">
        <Loader2 size={15} class="animate-spin" />
        {$t("library.loading")}
      </div>
    {:else if projectEntries.length === 0}
      <button
        class="text-base-content-faint hover:text-base-content-subtle w-full px-4 py-3 text-left text-xs transition"
        type="button"
        onclick={onAddProject}
      >
        {$t("projectManage.empty")}
      </button>
    {:else}
      {#each projectEntries as entry (entry.key)}
        {@render row(entry)}
      {/each}
    {/if}
  </div>
</div>
