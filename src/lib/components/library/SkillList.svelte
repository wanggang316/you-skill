<script lang="ts">
  import { untrack } from "svelte";
  import {
    Check,
    ChevronRight,
    Folder,
    Github,
    Loader2,
    RefreshCw,
    ScanSearch,
    Search,
    Zap,
  } from "@lucide/svelte";
  import SkillIcon from "$lib/components/SkillIcon.svelte";
  import IconButton from "$lib/components/ui/IconButton.svelte";
  import PrimaryActionButton from "$lib/components/ui/PrimaryActionButton.svelte";
  import SelectField from "$lib/components/ui/SelectField.svelte";
  import { t } from "$lib/i18n";
  import type { HubSkillView } from "$lib/api/hub";
  import { githubAvatarUrl, groupLibrarySkills } from "$lib/library-groups";
  import type { LibraryFilter, LibrarySource } from "$lib/navigation/app-shell";

  let {
    skills = [],
    totalCount = 0,
    selectedName = null,
    checkedNames = [],
    loading = false,
    checking = false,
    error = "",
    search = $bindable(""),
    filter = "all",
    source = "all",
    onSelect,
    onToggleChecked,
    onBatchInstall,
    onClearChecked,
    onRefresh,
    onScan,
    onFilterChange,
    onSourceChange,
  }: {
    skills?: HubSkillView[];
    totalCount?: number;
    selectedName?: string | null;
    /** Skills picked with Shift+click for a batch install. */
    checkedNames?: string[];
    loading?: boolean;
    /** Source update check in progress. */
    checking?: boolean;
    error?: string;
    search?: string;
    filter?: LibraryFilter;
    source?: LibrarySource;
    onSelect: (name: string) => void;
    onToggleChecked: (name: string) => void;
    onBatchInstall: () => void;
    onClearChecked: () => void;
    onRefresh: () => void;
    onScan: () => void;
    onFilterChange: (filter: LibraryFilter) => void;
    onSourceChange: (source: LibrarySource) => void;
  } = $props();

  const filters: LibraryFilter[] = ["all", "changed", "uninstalled"];
  const sources: LibrarySource[] = ["all", "local", "github", "none"];

  const groups = $derived(groupLibrarySkills(skills));

  /** Collapsed group keys, remembered per machine. */
  const COLLAPSED_STORAGE_KEY = "youskill.collapsedLibraryGroups";

  const readCollapsed = (): string[] => {
    try {
      const parsed: unknown = JSON.parse(localStorage.getItem(COLLAPSED_STORAGE_KEY) ?? "[]");
      return Array.isArray(parsed) ? parsed.filter((key) => typeof key === "string") : [];
    } catch {
      return [];
    }
  };

  let collapsed = $state<string[]>(readCollapsed());
  /** GitHub owners whose avatar failed to load; they show the GitHub icon instead. */
  let failedAvatars = $state<string[]>([]);

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

  // Selecting a skill elsewhere (deep links, imports) must not land in a hidden row.
  $effect(() => {
    const group = groups.find((item) => item.skills.some((skill) => skill.name === selectedName));
    if (!group) return;
    const current = untrack(() => collapsed);
    if (current.includes(group.key)) setCollapsed(current.filter((key) => key !== group.key));
  });

  function handleRowClick(event: MouseEvent, name: string) {
    if (event.shiftKey) {
      onToggleChecked(name);
      return;
    }
    onSelect(name);
  }
</script>

<div class="border-base-300 flex min-h-0 min-w-0 flex-col border-r">
  <header
    class="border-base-300 flex h-12 flex-none items-center justify-between border-b px-4"
    data-window-drag-region
  >
    <div class="flex min-w-0 items-baseline gap-2">
      <h1 class="text-base-content truncate text-[1.05rem] font-semibold tracking-[-0.02em]">
        {$t("library.title")}
      </h1>
      <span class="text-base-content-faint shrink-0 text-xs">
        {$t("library.count", { count: totalCount })}
      </span>
    </div>
    <div class="flex shrink-0 items-center gap-1.5">
      <IconButton
        variant="outline"
        onclick={onScan}
        title={$t("library.scan")}
        ariaLabel={$t("library.scan")}
        class="h-8 w-8 p-0"
      >
        <ScanSearch size={15} />
      </IconButton>
      <IconButton
        variant="outline"
        onclick={onRefresh}
        title={$t("library.refresh")}
        ariaLabel={$t("library.refresh")}
        class="h-8 w-8 p-0"
      >
        <RefreshCw size={15} class={loading || checking ? "animate-spin" : ""} />
      </IconButton>
    </div>
  </header>

  <div class="border-base-300 space-y-2 border-b px-3 py-2.5">
    <div class="relative">
      <Search class="text-base-content-subtle absolute top-1/2 left-3 -translate-y-1/2" size={14} />
      <input
        class="border-base-300 bg-base-200 text-base-content placeholder:text-base-content-subtle focus:border-base-300 h-8 w-full rounded-xl border pr-3 pl-8 text-[13px] focus:outline-none"
        placeholder={$t("library.search")}
        bind:value={search}
      />
    </div>
    <div class="flex items-center gap-1.5">
      <SelectField
        value={filter}
        className="min-w-0 flex-1"
        selectClassName="h-8 text-[12px]"
        onchange={(event) => onFilterChange(event.currentTarget.value as LibraryFilter)}
      >
        {#each filters as item}
          <option value={item}>{$t(`library.filter.${item}`)}</option>
        {/each}
      </SelectField>
      <SelectField
        value={source}
        className="min-w-0 flex-1"
        selectClassName="h-8 text-[12px]"
        onchange={(event) => onSourceChange(event.currentTarget.value as LibrarySource)}
      >
        {#each sources as item}
          <option value={item}>{$t(`library.source.${item}`)}</option>
        {/each}
      </SelectField>
    </div>
  </div>

  <div
    class="min-h-0 flex-1 overflow-y-auto py-1.5 [scrollbar-color:var(--scrollbar-thumb)_transparent] [scrollbar-width:thin]"
  >
    {#if error}
      <p class="text-error px-4 py-3 text-xs">{error}</p>
    {:else if loading && skills.length === 0}
      <div class="text-base-content-muted flex items-center justify-center gap-2 py-10 text-xs">
        <Loader2 size={16} class="animate-spin" />
        {$t("library.loading")}
      </div>
    {:else if skills.length === 0}
      <p class="text-base-content-muted px-4 py-8 text-center text-xs">
        {totalCount === 0 ? $t("library.empty") : $t("library.emptyFiltered")}
      </p>
    {:else}
      {#each groups as group (group.key)}
        {@const open = !collapsed.includes(group.key)}
        <button
          class="text-base-content-subtle hover:text-base-content flex w-full items-center gap-1.5 px-4 pt-2 pb-1 text-left text-[11px] transition"
          type="button"
          onclick={() => toggleGroup(group.key)}
          aria-expanded={open}
          title={group.path ?? undefined}
        >
          <ChevronRight size={12} class={`shrink-0 transition ${open ? "rotate-90" : ""}`} />
          {#if group.kind === "github"}
            {#if failedAvatars.includes(group.label)}
              <Github size={13} class="shrink-0" />
            {:else}
              <img
                class="bg-base-300 size-4 shrink-0 rounded-full"
                src={githubAvatarUrl(group.label)}
                alt=""
                loading="lazy"
                onerror={() => (failedAvatars = [...failedAvatars, group.label])}
              />
            {/if}
          {:else if group.kind === "folder"}
            <Folder size={13} class="shrink-0" />
          {:else}
            <Zap size={13} class="shrink-0" />
          {/if}
          <span class="min-w-0 flex-1 truncate font-medium">
            {group.kind === "none" ? $t("library.group.none") : group.label}
          </span>
          <span class="text-base-content-faint shrink-0">{group.skills.length}</span>
        </button>
        {#if open}
          {#each group.skills as skill (skill.name)}
            {@const selected = skill.name === selectedName}
            {@const checked = checkedNames.includes(skill.name)}
            <button
              class={`mx-1.5 flex w-[calc(100%-0.75rem)] flex-col gap-1 rounded-lg px-2.5 py-2 text-left transition select-none ${
                checked
                  ? "bg-primary/10 ring-primary/40 ring-1 ring-inset"
                  : selected
                    ? "bg-base-300"
                    : "hover:bg-base-200"
              }`}
              type="button"
              onclick={(event) => handleRowClick(event, skill.name)}
              aria-current={selected ? "true" : undefined}
              aria-pressed={checkedNames.length > 0 ? checked : undefined}
            >
              <div class="flex w-full items-center gap-2">
                <span class={`shrink-0 ${checked ? "text-primary" : "text-base-content-subtle"}`}>
                  {#if checked}
                    <Check size={13} />
                  {:else}
                    <SkillIcon source={skill.source} />
                  {/if}
                </span>
                <span class="text-base-content min-w-0 flex-1 truncate text-[13px] font-medium">
                  {skill.name}
                </span>
                {#if skill.hasDrift}
                  <span class="tag tag-warning shrink-0">{$t("library.tag.changed")}</span>
                {:else if skill.installs.length === 0}
                  <span class="tag tag-neutral shrink-0">{$t("library.tag.uninstalled")}</span>
                {/if}
              </div>
              {#if skill.description}
                <p class="text-base-content-subtle line-clamp-1 pl-[1.3rem] text-[11px]">
                  {skill.description}
                </p>
              {/if}
            </button>
          {/each}
        {/if}
      {/each}
    {/if}
  </div>

  {#if checkedNames.length > 0}
    <div class="border-base-300 flex flex-none items-center gap-2 border-t px-3 py-2.5">
      <PrimaryActionButton className="flex-[2] py-1.5 text-[13px]" onclick={onBatchInstall}>
        {$t("library.batchInstall", { count: checkedNames.length })}
      </PrimaryActionButton>
      <button
        class="border-base-300 text-base-content hover:bg-base-200 flex-1 rounded-xl border px-3 py-1.5 text-[13px] transition"
        type="button"
        onclick={onClearChecked}
      >
        {$t("common.cancel")}
      </button>
    </div>
  {/if}
</div>
