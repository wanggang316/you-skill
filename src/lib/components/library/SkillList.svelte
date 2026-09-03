<script lang="ts">
  import {
    FileArchive,
    Folder,
    Github,
    Loader2,
    Plus,
    RefreshCw,
    ScanSearch,
    Search,
    Zap,
  } from "@lucide/svelte";
  import IconButton from "$lib/components/ui/IconButton.svelte";
  import SelectField from "$lib/components/ui/SelectField.svelte";
  import { t } from "$lib/i18n";
  import { scopedHasDrift, type HubSkillView, type ScopeRef } from "$lib/api/hub";
  import type { LibraryFilter } from "$lib/navigation/app-shell";

  let {
    skills = [],
    totalCount = 0,
    selectedName = null,
    loading = false,
    error = "",
    search = $bindable(""),
    filter = "all",
    filters = ["all", "changed", "uninstalled"],
    scope = null,
    title = "",
    emptyText = null,
    onSelect,
    onRefresh,
    onScan,
    onFilterChange,
    onAddSkill = null,
  }: {
    skills?: HubSkillView[];
    totalCount?: number;
    selectedName?: string | null;
    loading?: boolean;
    error?: string;
    search?: string;
    filter?: LibraryFilter;
    filters?: LibraryFilter[];
    /** Scope the list is narrowed to; the changed tag then reflects that scope only. */
    scope?: ScopeRef | null;
    /** Header title; defaults to the library title. */
    title?: string;
    /** Shown instead of the library empty text when the list has no skills at all. */
    emptyText?: string | null;
    onSelect: (name: string) => void;
    onRefresh: () => void;
    onScan: () => void;
    onFilterChange: (filter: LibraryFilter) => void;
    /** When set, a button to add library skills to the current scope is shown. */
    onAddSkill?: (() => void) | null;
  } = $props();
</script>

<div class="border-base-300 flex min-h-0 min-w-0 flex-col border-r">
  <header
    class="border-base-300 flex h-12 flex-none items-center justify-between border-b px-4"
    data-window-drag-region
  >
    <h1 class="text-base-content truncate text-[1.05rem] font-semibold tracking-[-0.02em]">
      {title || $t("library.title")}
    </h1>
    <span class="text-base-content-faint text-xs">{$t("library.count", { count: totalCount })}</span
    >
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
        selectClassName="h-8 w-full text-[12px]"
        onchange={(event) => onFilterChange(event.currentTarget.value as LibraryFilter)}
      >
        {#each filters as item}
          <option value={item}>{$t(`library.filter.${item}`)}</option>
        {/each}
      </SelectField>
      {#if onAddSkill}
        <IconButton
          variant="outline"
          onclick={onAddSkill}
          title={$t("scope.addSkill")}
          ariaLabel={$t("scope.addSkill")}
          class="h-8 w-8 p-0"
        >
          <Plus size={15} />
        </IconButton>
      {/if}
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
        <RefreshCw size={15} class={loading ? "animate-spin" : ""} />
      </IconButton>
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
        {totalCount === 0 ? (emptyText ?? $t("library.empty")) : $t("library.emptyFiltered")}
      </p>
    {:else}
      {#each skills as skill (skill.name)}
        {@const selected = skill.name === selectedName}
        <button
          class={`mx-1.5 flex w-[calc(100%-0.75rem)] flex-col gap-1 rounded-lg px-2.5 py-2 text-left transition ${
            selected ? "bg-base-300" : "hover:bg-base-200"
          }`}
          type="button"
          onclick={() => onSelect(skill.name)}
          aria-current={selected ? "true" : undefined}
        >
          <div class="flex w-full items-center gap-2">
            <span class="text-base-content-subtle shrink-0">
              {#if skill.source.type === "github"}
                <Github size={13} />
              {:else if skill.source.type === "zip"}
                <FileArchive size={13} />
              {:else if skill.source.type === "folder"}
                <Folder size={13} />
              {:else}
                <Zap size={13} />
              {/if}
            </span>
            <span class="text-base-content min-w-0 flex-1 truncate text-[13px] font-medium">
              {skill.name}
            </span>
            {#if scope ? scopedHasDrift(skill, scope) : skill.hasDrift}
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
  </div>
</div>
