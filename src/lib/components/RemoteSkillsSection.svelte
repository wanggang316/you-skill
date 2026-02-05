<script>
  import { ChevronDown, CloudDownload, RefreshCw, Search } from 'lucide-svelte'

  let {
    remoteQuery = $bindable(),
    installAgent = $bindable(),
    installGlobal = $bindable(),
    agents,
    remoteLoading,
    remoteSkills,
    remoteError,
    installLog,
    installingSkill,
    remoteHasMore,
    onSearch,
    onLoadMore,
    onInstall
  } = $props()
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
          placeholder="搜索远程技能（名称或仓库）"
          bind:value={remoteQuery}
          onkeydown={(event) => event.key === 'Enter' && onSearch()}
        />
      </div>
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
        全局安装
      </label>
      <button
        class="rounded-xl border border-[var(--base-300)] p-2 text-sm text-[var(--base-content)]"
        onclick={onSearch}
        title="搜索"
        type="button"
      >
        <Search size={16} />
      </button>
    </div>
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
        正在加载技能库...
      </div>
    {:else if remoteSkills.length === 0}
      <div
        class="rounded-2xl border border-dashed border-[var(--base-300)] bg-[var(--base-100)] p-6 text-center text-sm text-[var(--base-content-muted)]"
      >
        暂无技能数据
      </div>
    {:else}
      {#each remoteSkills as skill}
        <div class="rounded-2xl border border-[var(--base-300)] bg-[var(--base-100)] p-4">
          <div class="flex flex-wrap items-center justify-between gap-3">
            <div>
              <p class="text-base font-semibold">{skill.name}</p>
              <p class="text-xs text-[var(--base-content-muted)]">{skill.source}</p>
              <p class="mt-1 text-xs text-[var(--base-content-faint)]">
                {skill.installs} installs
              </p>
            </div>
            <button
              class="rounded-xl bg-[var(--primary)] p-2 text-xs text-[var(--primary-content)]"
              onclick={() => onInstall(skill)}
              disabled={installingSkill === skill.id}
              title="一键安装"
              type="button"
            >
              {#if installingSkill === skill.id}
                <RefreshCw size={14} class="animate-spin" />
              {:else}
                <CloudDownload size={14} />
              {/if}
            </button>
          </div>
        </div>
      {/each}
    {/if}
  </div>

  <div class="flex items-center justify-center">
    <button
      class="rounded-xl border border-[var(--base-300)] p-2 text-sm text-[var(--base-content)]"
      onclick={onLoadMore}
      disabled={!remoteHasMore || remoteLoading}
      title={remoteHasMore ? '加载更多' : '没有更多了'}
      type="button"
    >
      {#if remoteLoading}
        <RefreshCw size={16} class="animate-spin" />
      {:else}
        <ChevronDown size={16} />
      {/if}
    </button>
  </div>
</section>
