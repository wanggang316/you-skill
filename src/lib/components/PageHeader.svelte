<script>
  import { ChevronLeft, HardDrive, Plus, Settings } from '@lucide/svelte'

  const { currentView, activeTab, onChangeView, onChangeTab } = $props()
</script>

<header class="border-b border-[var(--base-300)] bg-[var(--base-100)]">
  <div class="mx-auto flex max-w-6xl items-center justify-between px-6 py-4">
    <div class="flex items-center gap-3">
      <div
        class="rounded-xl bg-[var(--primary)] p-2 text-[var(--primary-content)]"
      >
        <HardDrive size={18} />
      </div>
      <div>
        <p class="text-lg font-semibold">Skill Kit</p>
        <p class="text-xs text-[var(--base-content-muted)]">本地管理与一键安装</p>
      </div>
    </div>
    <div class="flex items-center gap-3 text-sm">
      {#if currentView === 'list'}
        <div class="flex items-center gap-2 rounded-full bg-[var(--base-200)] p-1">
          <button
            class={`rounded-full px-4 py-2 ${activeTab === 'local' ? 'bg-[var(--base-100)] text-[var(--base-content)] shadow-sm' : 'text-[var(--base-content-muted)]'}`}
            onclick={() => onChangeTab('local')}
            type="button"
          >
            本地技能
          </button>
          <button
            class={`rounded-full px-4 py-2 ${activeTab === 'remote' ? 'bg-[var(--base-100)] text-[var(--base-content)] shadow-sm' : 'text-[var(--base-content-muted)]'}`}
            onclick={() => onChangeTab('remote')}
            type="button"
          >
            远程技能库
          </button>
        </div>
        <button
          class="flex items-center gap-2 rounded-xl border border-[var(--base-300)] px-3 py-2 text-sm text-[var(--base-content)]"
          onclick={() => onChangeView('settings')}
          title="设置"
          type="button"
        >
          <Settings size={16} />
          设置
        </button>
        <button
          class="flex items-center gap-2 rounded-xl bg-[var(--primary)] px-3 py-2 text-sm text-[var(--primary-content)]"
          onclick={() => onChangeView('add')}
          title="新增"
          type="button"
        >
          <Plus size={16} />
          新增
        </button>
      {:else}
        <button
          class="flex items-center gap-2 rounded-xl border border-[var(--base-300)] px-3 py-2 text-sm text-[var(--base-content)]"
          onclick={() => onChangeView('list')}
          title="返回"
          type="button"
        >
          <ChevronLeft size={16} />
          返回
        </button>
      {/if}
    </div>
  </div>
</header>
