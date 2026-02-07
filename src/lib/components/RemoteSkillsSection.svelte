<script>
  import { ChevronDown, CloudDownload, ExternalLink, Loader2, RefreshCw, Search, Check, ArrowUpDown } from '@lucide/svelte'
  import IconButton from './IconButton.svelte'
  import { t } from '../i18n'

  let {
    remoteQuery = $bindable(),
    installAgent = $bindable(),
    installGlobal = $bindable(),
    agents,
    localSkills = [],
    remoteLoading,
    remoteSkills,
    remoteError,
    installLog,
    installingSkill,
    isDownloading,
    remoteHasMore,
    remoteTotal = 0,
    remoteSortBy = $bindable('star_count'),
    remoteSortOrder = $bindable('desc'),
    onSearch,
    onLoadMore,
    onInstall,
    onOpenUrl,
    onViewSkill,
    onSortChange
  } = $props()

  const sortOptions = [
    { value: 'star_count_desc', label: 'Most Stars' },
    { value: 'star_count_asc', label: 'Least Stars' },
    { value: 'created_at_desc', label: 'Newest' },
    { value: 'created_at_asc', label: 'Oldest' },
    { value: 'name_asc', label: 'Name A-Z' },
    { value: 'name_desc', label: 'Name Z-A' }
  ]

  function handleSortChange(event) {
    const value = event.target.value
    const [sortBy, sortOrder] = value.split('_')
    onSortChange(sortBy, sortOrder)
  }

  function handleOpenUrl(skill) {
    if (skill.url) {
      onOpenUrl(skill.url)
    }
  }

  // Check if skill is already installed locally
  function isInstalled(skill) {
    return localSkills.some(local => local.name === skill.name)
  }
</script>

<section class="space-y-6">
  <div class="rounded-2xl border border-[var(--base-300)] bg-[var(--base-100)] p-4">
    <div class="flex flex-wrap items-center gap-3">
      <div class="relative flex-1">
        <Search
          class="absolute left-3 top-3 text-[var(--base-content-subtle)]"
          size={16}
        />
        <input
          class="w-full rounded-xl border border-[var(--base-300)] bg-[var(--base-200)] px-9 py-2 text-sm text-[var(--base-content)] placeholder:text-[var(--base-content-subtle)] focus:border-[var(--base-300)] focus:outline-none"
          placeholder={$t('remote.search.placeholder')}
          bind:value={remoteQuery}
          onkeydown={(event) => event.key === 'Enter' && onSearch()}
        />
      </div>
      <select
        class="rounded-xl border border-[var(--base-300)] bg-[var(--base-100)] px-3 py-2 text-sm text-[var(--base-content)]"
        value={`${remoteSortBy}_${remoteSortOrder}`}
        onchange={handleSortChange}
      >
        {#each sortOptions as option}
          <option value={option.value}>{option.label}</option>
        {/each}
      </select>
      <select
        class="rounded-xl border border-[var(--base-300)] bg-[var(--base-100)] px-3 py-2 text-sm text-[var(--base-content)]"
        bind:value={installAgent}
      >
        {#each agents as agent}
          <option value={agent.id}>{agent.display_name}</option>
        {/each}
      </select>
      <label class="flex items-center gap-2 text-xs text-[var(--base-content-muted)]">
        <input type="checkbox" bind:checked={installGlobal} />
        {$t('remote.installGlobal')}
      </label>
      <IconButton
        variant="outline"
        onclick={onSearch}
        title={$t('remote.search')}
        ariaLabel={$t('remote.search')}
      >
        <Search size={16} />
      </IconButton>
    </div>
    {#if remoteTotal > 0}
      <p class="mt-2 text-xs text-[var(--base-content-muted)]">
        {$t('remote.total', { count: remoteTotal })}
      </p>
    {/if}
    {#if remoteError}
      <p class="mt-3 text-sm text-[var(--error)]">{remoteError}</p>
    {/if}
    {#if installLog}
      <div
        class="mt-3 whitespace-pre-wrap rounded-lg bg-[var(--base-200)] px-3 py-2 text-xs text-[var(--base-content-muted)]"
      >
        {installLog}
      </div>
    {/if}
  </div>

  <div class="space-y-3">
    {#if remoteLoading && remoteSkills.length === 0}
      <div
        class="rounded-2xl border border-dashed border-[var(--base-300)] bg-[var(--base-100)] p-6 text-center text-sm text-[var(--base-content-muted)]"
      >
        {$t('remote.loading')}
      </div>
    {:else if remoteSkills.length === 0}
      <div
        class="rounded-2xl border border-dashed border-[var(--base-300)] bg-[var(--base-100)] p-6 text-center text-sm text-[var(--base-content-muted)]"
      >
        {$t('remote.empty')}
      </div>
    {:else}
      {#each remoteSkills as skill}
        {@const installed = isInstalled(skill)}
        {@const isBusy = installingSkill === skill.id || isDownloading}
        <div
          class="rounded-2xl border border-[var(--base-300)] bg-[var(--base-100)] p-4 transition hover:bg-[var(--base-200)] hover:shadow-sm cursor-pointer"
          onclick={() => onViewSkill(skill)}
          onkeydown={(e) => (e.key === 'Enter' || e.key === ' ') && onViewSkill(skill)}
          role="button"
          tabindex="0"
          aria-label={`View ${skill.name}`}
        >
          <div class="flex flex-wrap items-center justify-between gap-3">
            <div class="min-w-0 flex-1">
              <div class="flex items-center gap-2">
                <p class="text-base font-semibold truncate">{skill.name} <span class="text-[var(--base-content-muted)] font-normal">({skill.source})</span></p>
                {#if installed}
                  <span class="inline-flex items-center gap-1 rounded-full bg-[var(--success-bg)] px-2 py-0.5 text-xs text-[var(--success)]">
                    <Check size={12} />
                    {$t('remote.installed')}
                  </span>
                {/if}
              </div>
              <p class="mt-1 text-xs text-[var(--base-content-faint)]">
                {$t('remote.installs', { count: skill.installs })}
              </p>
            </div>
            <div class="flex items-center gap-2">
              {#if skill.url}
                <IconButton
                  variant="outline"
                  onclick={(e) => {
                    e?.stopPropagation()
                    handleOpenUrl(skill)
                  }}
                  disabled={isBusy}
                  title={$t('remote.openUrl')}
                  ariaLabel={$t('remote.openUrl')}
                >
                  <ExternalLink size={14} />
                </IconButton>
              {/if}
              {#if installed}
                <span class="text-sm text-[var(--base-content-muted)]">{$t('remote.installed')}</span>
              {:else}
                <IconButton
                  variant="primary"
                  onclick={(e) => {
                    e?.stopPropagation()
                    onInstall(skill)
                  }}
                  disabled={isBusy}
                  title={installingSkill === skill.id ? $t('remote.downloading') : $t('remote.install')}
                  ariaLabel={installingSkill === skill.id ? $t('remote.downloading') : $t('remote.install')}
                >
                  {#if installingSkill === skill.id}
                    <Loader2 size={14} class="animate-spin" />
                  {:else}
                    <CloudDownload size={14} />
                  {/if}
                </IconButton>
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
      title={remoteHasMore ? $t('remote.loadMore') : $t('remote.noMore')}
      ariaLabel={remoteHasMore ? $t('remote.loadMore') : $t('remote.noMore')}
    >
      {#if remoteLoading}
        <RefreshCw size={16} class="animate-spin" />
      {:else}
        <ChevronDown size={16} />
      {/if}
    </IconButton>
  </div>
</section>
