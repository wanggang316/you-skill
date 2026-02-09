<script>
  import { ChevronDown, Loader2, RefreshCw, Search, Check } from "@lucide/svelte";
  import IconButton from "./ui/IconButton.svelte";
  import { t } from "../i18n";

  let {
    remoteQuery = $bindable(),
    localSkills = [],
    remoteLoading,
    remoteSkills,
    remoteError,
    installLog,
    installingSkill,
    isDownloading,
    remoteHasMore,
    remoteTotal = 0,
    remoteSortBy = $bindable("heat_score"),
    remoteSortOrder = $bindable("desc"),
    onSearch,
    onLoadMore,
    onInstall,
    onViewSkill,
    onSortChange,
    onRefresh,
  } = $props();

  let searchTimeout = $state(null);

  const sortOptions = [
    { value: "heat_score", label: "Most Popular" },
    { value: "star_count", label: "Most Stars" },
  ];

  function handleSortChange() {
    // remoteSortBy is updated via bind:value, always use 'desc'
    onSortChange(remoteSortBy, "desc");
  }

  function handleSearchInput() {
    if (searchTimeout) {
      clearTimeout(searchTimeout);
    }
    searchTimeout = setTimeout(() => {
      onSearch();
    }, 500);
  }

  // Check if skill is already installed locally
  function isInstalled(skill) {
    return localSkills.some((local) => local.name === skill.name);
  }
</script>

<section class="space-y-6">
  <div class="border-base-300 bg-base-100 rounded-2xl border p-4">
    <div class="flex flex-wrap items-center gap-3">
      <div class="relative flex-1">
        <Search
          class="text-base-content-subtle absolute top-1/2 left-3 -translate-y-1/2"
          size={16}
        />
        <input
          class="border-base-300 bg-base-200 text-base-content placeholder:text-base-content-subtle focus:border-base-300 h-9 w-full rounded-xl border px-9 text-sm focus:outline-none"
          placeholder={$t("remote.search.placeholder")}
          bind:value={remoteQuery}
          oninput={handleSearchInput}
        />
      </div>
      <select
        class="border-base-300 bg-base-100 text-base-content focus:border-base-300 h-9 cursor-pointer rounded-xl border px-3 text-sm focus:outline-none"
        bind:value={remoteSortBy}
        onchange={handleSortChange}
      >
        {#each sortOptions as option}
          <option value={option.value}>{option.label}</option>
        {/each}
      </select>
      <IconButton
        variant="outline"
        onclick={onRefresh}
        title={$t("local.refresh")}
        ariaLabel={$t("local.refresh")}
        class="h-9 w-9"
      >
        <RefreshCw size={16} class={remoteLoading ? "animate-spin" : ""} />
      </IconButton>
    </div>
    {#if remoteTotal > 0}
      <p class="text-base-content-muted mt-2 text-xs">
        {$t("remote.total", { count: remoteTotal })}
      </p>
    {/if}
    {#if remoteError}
      <p class="text-error mt-3 text-sm">{remoteError}</p>
    {/if}
    {#if installLog}
      <div
        class="bg-base-200 text-base-content-muted mt-3 rounded-lg px-3 py-2 text-xs whitespace-pre-wrap"
      >
        {installLog}
      </div>
    {/if}
  </div>

  <div class="space-y-3">
    {#if remoteLoading && remoteSkills.length === 0}
      <div
        class="border-base-300 bg-base-100 text-base-content-muted rounded-2xl border border-dashed p-6 text-center text-sm"
      >
        {$t("remote.loading")}
      </div>
    {:else if remoteSkills.length === 0}
      <div
        class="border-base-300 bg-base-100 text-base-content-muted rounded-2xl border border-dashed p-6 text-center text-sm"
      >
        {$t("remote.empty")}
      </div>
    {:else}
      {#each remoteSkills as skill}
        {@const installed = isInstalled(skill)}
        {@const isBusy = installingSkill === skill.id || isDownloading}
        <div
          class="border-base-300 bg-base-100 hover:bg-base-200 cursor-pointer rounded-2xl border p-4 transition hover:shadow-sm"
          onclick={() => onViewSkill(skill)}
          onkeydown={(e) => (e.key === "Enter" || e.key === " ") && onViewSkill(skill)}
          role="button"
          tabindex="0"
          aria-label={`View ${skill.name}`}
        >
          <div class="flex flex-wrap items-center justify-between gap-3">
            <div class="min-w-0 flex-1">
              <div class="flex items-center gap-2">
                <p class="truncate text-base font-semibold">
                  {skill.name}
                  <span class="text-base-content-muted font-normal">({skill.source})</span>
                </p>
              </div>
              <p class="text-base-content-faint mt-1 text-xs">
                {$t("remote.stars", { count: skill.star_count })}
              </p>
            </div>
            <div
              class="flex items-center gap-2"
              onclick={(e) => e.stopPropagation()}
              onkeydown={(e) => e.stopPropagation()}
              role="presentation"
            >
              {#if installed}
                <button
                  class="border-success text-success inline-flex cursor-default items-center rounded-lg border bg-transparent px-2 py-0.5 text-xs"
                  type="button"
                  disabled
                >
                  <Check size={12} class="mr-1.5" />
                  {$t("remote.installed")}
                </button>
              {:else}
                <button
                  class="border-primary text-primary hover:bg-primary hover:text-primary-content inline-flex items-center rounded-lg border bg-transparent px-2 py-0.5 text-xs transition disabled:cursor-not-allowed disabled:opacity-50"
                  onclick={(e) => {
                    e?.stopPropagation();
                    onInstall(skill);
                  }}
                  disabled={isBusy}
                  type="button"
                >
                  {#if installingSkill === skill.id}
                    <Loader2 size={12} class="mr-1.5 animate-spin" />
                    {$t("remote.downloading")}
                  {:else}
                    {$t("remote.install")}
                  {/if}
                </button>
              {/if}
            </div>
          </div>
        </div>
      {/each}
    {/if}
  </div>

  <div class="flex items-center justify-center">
    <IconButton
      variant="outline"
      onclick={onLoadMore}
      disabled={!remoteHasMore || remoteLoading}
      title={remoteHasMore ? $t("remote.loadMore") : $t("remote.noMore")}
      ariaLabel={remoteHasMore ? $t("remote.loadMore") : $t("remote.noMore")}
    >
      {#if remoteLoading}
        <RefreshCw size={16} class="animate-spin" />
      {:else}
        <ChevronDown size={16} />
      {/if}
    </IconButton>
  </div>
</section>
