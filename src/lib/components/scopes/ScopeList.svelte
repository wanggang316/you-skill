<script lang="ts">
  import { untrack } from "svelte";
  import {
    AlertTriangle,
    ChevronRight,
    Folder,
    FolderOpen,
    FolderPlus,
    Loader2,
    RefreshCw,
    UserRound,
  } from "@lucide/svelte";
  import DropdownMenu, { type MenuItem } from "$lib/components/ui/DropdownMenu.svelte";
  import IconButton from "$lib/components/ui/IconButton.svelte";
  import { t } from "$lib/i18n";
  import type { UserWorkspace } from "$lib/api/user-projects";
  import type { ScopeEntry } from "$lib/scopes";

  let {
    entries = [],
    workspaces = [],
    selectedKey = null,
    loading = false,
    error = "",
    onSelect,
    onAddWorkspace,
    onRescanWorkspace,
    onRemoveWorkspace,
    onRemoveProject,
    onForgetProject,
    onRefresh,
  }: {
    entries?: ScopeEntry[];
    workspaces?: UserWorkspace[];
    selectedKey?: string | null;
    loading?: boolean;
    error?: string;
    onSelect: (entry: ScopeEntry) => void;
    onAddWorkspace: () => void;
    onRescanWorkspace: (workspace: UserWorkspace) => void;
    onRemoveWorkspace: (workspace: UserWorkspace) => void;
    /** Take a registered project off the list; its files stay. */
    onRemoveProject: (entry: ScopeEntry) => void;
    /** Drop a project whose folder is gone, records included. */
    onForgetProject: (entry: ScopeEntry) => void;
    onRefresh: () => void;
  } = $props();

  type Group = {
    key: string;
    label: string | null;
    workspace: UserWorkspace | null;
    entries: ScopeEntry[];
  };

  /** Collapsed workspace keys, remembered per machine. */
  const COLLAPSED_STORAGE_KEY = "youskill.collapsedWorkspaces";

  const readCollapsed = (): string[] => {
    try {
      const parsed: unknown = JSON.parse(localStorage.getItem(COLLAPSED_STORAGE_KEY) ?? "[]");
      return Array.isArray(parsed) ? parsed.filter((key) => typeof key === "string") : [];
    } catch {
      return [];
    }
  };

  let collapsed = $state<string[]>(readCollapsed());

  function setCollapsed(keys: string[]) {
    collapsed = keys;
    try {
      localStorage.setItem(COLLAPSED_STORAGE_KEY, JSON.stringify(keys));
    } catch {
      // Remembering the state is best effort.
    }
  }

  const toggleGroup = (key: string) =>
    setCollapsed(
      collapsed.includes(key) ? collapsed.filter((item) => item !== key) : [...collapsed, key]
    );

  function projectMenu(entry: ScopeEntry): MenuItem[] {
    if (entry.kind !== "project") return [];
    if (entry.missing) {
      return [
        { label: $t("scope.project.forget"), danger: true, onSelect: () => onForgetProject(entry) },
      ];
    }
    if (!entry.unregistered) {
      return [
        { label: $t("scope.project.remove"), danger: true, onSelect: () => onRemoveProject(entry) },
      ];
    }
    return [];
  }

  const userEntries = $derived(entries.filter((entry) => entry.kind === "user"));
  const projectEntries = $derived(entries.filter((entry) => entry.kind === "project"));

  const groups = $derived.by((): Group[] => {
    const known = new Set(workspaces.map((workspace) => workspace.path));
    const result: Group[] = workspaces.map((workspace) => ({
      key: `workspace:${workspace.path}`,
      label: workspace.name,
      workspace,
      entries: projectEntries.filter((entry) => entry.workspacePath === workspace.path),
    }));
    const rest = projectEntries.filter(
      (entry) => !entry.workspacePath || !known.has(entry.workspacePath)
    );
    if (rest.length > 0) {
      result.push({
        key: "other",
        label: workspaces.length > 0 ? $t("workspace.other") : null,
        workspace: null,
        entries: rest,
      });
    }
    return result;
  });

  // Selecting a project elsewhere (library links, deep links) must not land in a hidden row.
  $effect(() => {
    const group = groups.find((item) => item.entries.some((entry) => entry.key === selectedKey));
    if (!group) return;
    const current = untrack(() => collapsed);
    if (current.includes(group.key)) setCollapsed(current.filter((key) => key !== group.key));
  });
</script>

{#snippet row(entry: ScopeEntry)}
  {@const selected = entry.key === selectedKey}
  {@const menu = projectMenu(entry)}
  <div class="group relative mx-1.5 mb-0.5">
    <button
      class={`flex w-full items-start gap-2.5 rounded-xl px-2.5 py-2 text-left transition ${
        selected ? "bg-base-300" : "hover:bg-base-200"
      } ${menu.length > 0 ? "pr-10" : ""}`}
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
          {#if entry.missing}
            <span class="text-error shrink-0" title={$t("projects.missing")}>
              <AlertTriangle size={12} />
            </span>
          {/if}
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
        <span class="text-base-content-faint mt-0.5 shrink-0 text-[11px]">
          {entry.skills.length}
        </span>
      {/if}
    </button>
    {#if menu.length > 0}
      <div
        class={`absolute top-1.5 right-2 transition ${
          entry.missing ? "" : "opacity-0 group-hover:opacity-100 focus-within:opacity-100"
        }`}
      >
        <DropdownMenu label={$t("scope.project.actions")} items={menu} />
      </div>
    {/if}
  </div>
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

    <div class="flex items-center justify-between gap-2 px-4 pt-3 pb-1.5">
      <p class="text-base-content-subtle text-[11px] font-medium tracking-wide uppercase">
        {$t("sidebar.projects")}
      </p>
      <div class="flex shrink-0 items-center gap-0.5">
        <button
          class="text-base-content-subtle hover:bg-base-300 hover:text-base-content inline-flex size-6 items-center justify-center rounded-md transition"
          type="button"
          onclick={onAddWorkspace}
          title={$t("workspace.addTitle")}
          aria-label={$t("workspace.addTitle")}
        >
          <FolderPlus size={14} />
        </button>
      </div>
    </div>

    {#if loading && projectEntries.length === 0}
      <div class="text-base-content-muted flex items-center justify-center gap-2 py-6 text-xs">
        <Loader2 size={15} class="animate-spin" />
        {$t("library.loading")}
      </div>
    {:else if groups.length === 0}
      <button
        class="text-base-content-faint hover:text-base-content-subtle w-full px-4 py-3 text-left text-xs transition"
        type="button"
        onclick={onAddWorkspace}
      >
        {$t("workspace.empty")}
      </button>
    {:else}
      {#each groups as group (group.key)}
        {@const open = !collapsed.includes(group.key)}
        {#if group.label}
          <div class="flex items-center justify-between gap-2 px-4 pt-2 pb-1">
            <button
              class="text-base-content-subtle hover:text-base-content flex min-w-0 flex-1 items-center gap-1 text-left text-[11px] transition"
              type="button"
              onclick={() => toggleGroup(group.key)}
              aria-expanded={open}
              title={group.workspace?.path}
            >
              <ChevronRight size={12} class={`shrink-0 transition ${open ? "rotate-90" : ""}`} />
              <span class="truncate">{group.label}</span>
              {#if !open}
                <span class="text-base-content-faint shrink-0">{group.entries.length}</span>
              {/if}
            </button>
            {#if group.workspace}
              {@const workspace = group.workspace}
              <DropdownMenu
                label={$t("workspace.actions")}
                items={[
                  {
                    label: $t("workspace.rescan"),
                    onSelect: () => onRescanWorkspace(workspace),
                  },
                  {
                    label: $t("workspace.remove"),
                    danger: true,
                    onSelect: () => onRemoveWorkspace(workspace),
                  },
                ]}
              />
            {/if}
          </div>
        {/if}
        {#if open || !group.label}
          {#each group.entries as entry (entry.key)}
            {@render row(entry)}
          {/each}
          {#if group.entries.length === 0}
            <p class="text-base-content-faint px-4 py-1.5 text-[11px]">
              {$t("workspace.emptyGroup")}
            </p>
          {/if}
        {/if}
      {/each}
    {/if}
  </div>
</div>
