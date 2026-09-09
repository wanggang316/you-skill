<script lang="ts">
  import { FileText, Loader2, Plus, RefreshCw, Search } from "@lucide/svelte";
  import IconButton from "$lib/components/ui/IconButton.svelte";
  import SelectField from "$lib/components/ui/SelectField.svelte";
  import { t } from "$lib/i18n";
  import type { InstructionView } from "$lib/api/instructions";
  import type { LibraryFilter } from "$lib/navigation/app-shell";

  let {
    items = [],
    totalCount = 0,
    selectedName = null,
    loading = false,
    error = "",
    search = $bindable(""),
    filter = "all",
    onSelect,
    onRefresh,
    onImport,
    onFilterChange,
  }: {
    items?: InstructionView[];
    totalCount?: number;
    selectedName?: string | null;
    loading?: boolean;
    error?: string;
    search?: string;
    filter?: LibraryFilter;
    onSelect: (name: string) => void;
    onRefresh: () => void;
    onImport: () => void;
    onFilterChange: (filter: LibraryFilter) => void;
  } = $props();

  const filters: LibraryFilter[] = ["all", "changed", "uninstalled"];
</script>

<div class="border-base-300 flex min-h-0 min-w-0 flex-col border-r">
  <header
    class="border-base-300 flex h-12 flex-none items-center justify-between border-b px-4"
    data-window-drag-region
  >
    <h1 class="text-base-content truncate text-[1.05rem] font-semibold tracking-[-0.02em]">
      {$t("instructions.title")}
    </h1>
    <span class="text-base-content-faint text-xs">
      {$t("instructions.count", { count: totalCount })}
    </span>
  </header>

  <div class="border-base-300 space-y-2 border-b px-3 py-2.5">
    <div class="relative">
      <Search class="text-base-content-subtle absolute top-1/2 left-3 -translate-y-1/2" size={14} />
      <input
        class="border-base-300 bg-base-200 text-base-content placeholder:text-base-content-subtle focus:border-base-300 h-8 w-full rounded-xl border pr-3 pl-8 text-[13px] focus:outline-none"
        placeholder={$t("instructions.search")}
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
      <IconButton
        variant="outline"
        onclick={onImport}
        title={$t("instructions.import")}
        ariaLabel={$t("instructions.import")}
        class="h-8 w-8 p-0"
      >
        <Plus size={15} />
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
    {:else if loading && items.length === 0}
      <div class="text-base-content-muted flex items-center justify-center gap-2 py-10 text-xs">
        <Loader2 size={16} class="animate-spin" />
        {$t("instructions.loading")}
      </div>
    {:else if items.length === 0}
      <p class="text-base-content-muted px-4 py-8 text-center text-xs">
        {totalCount === 0 ? $t("instructions.empty") : $t("instructions.emptyFiltered")}
      </p>
    {:else}
      {#each items as item (item.name)}
        {@const selected = item.name === selectedName}
        <button
          class={`mx-1.5 flex w-[calc(100%-0.75rem)] flex-col gap-1 rounded-lg px-2.5 py-2 text-left transition ${
            selected ? "bg-base-300" : "hover:bg-base-200"
          }`}
          type="button"
          onclick={() => onSelect(item.name)}
          aria-current={selected ? "true" : undefined}
        >
          <div class="flex w-full items-center gap-2">
            <span class="text-base-content-subtle shrink-0"><FileText size={13} /></span>
            <span class="text-base-content min-w-0 flex-1 truncate text-[13px] font-medium">
              {item.name}
            </span>
            {#if item.hasDrift}
              <span class="tag tag-warning shrink-0">{$t("library.tag.changed")}</span>
            {:else if item.installs.length === 0}
              <span class="tag tag-neutral shrink-0">{$t("library.tag.uninstalled")}</span>
            {/if}
          </div>
          {#if item.description}
            <p class="text-base-content-subtle line-clamp-1 pl-[1.3rem] text-[11px]">
              {item.description}
            </p>
          {/if}
        </button>
      {/each}
    {/if}
  </div>
</div>
