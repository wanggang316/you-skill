<script>
  import { ChevronLeft, HardDrive, Plus, Settings } from '@lucide/svelte'
  import { t } from '../i18n'

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
        <p class="text-xs text-[var(--base-content-muted)]">
          {$t('header.subtitle')}
        </p>
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
            {$t('header.localTab')}
          </button>
          <button
            class={`rounded-full px-4 py-2 ${activeTab === 'remote' ? 'bg-[var(--base-100)] text-[var(--base-content)] shadow-sm' : 'text-[var(--base-content-muted)]'}`}
            onclick={() => onChangeTab('remote')}
            type="button"
          >
            {$t('header.remoteTab')}
          </button>
        </div>
        <button
          class="flex items-center gap-2 rounded-xl border border-[var(--base-300)] px-3 py-2 text-sm text-[var(--base-content)]"
          onclick={() => onChangeView('settings')}
          title={$t('header.settings')}
          type="button"
        >
          <Settings size={16} />
          {$t('header.settings')}
        </button>
        <button
          class="flex items-center gap-2 rounded-xl bg-[var(--primary)] px-3 py-2 text-sm text-[var(--primary-content)]"
          onclick={() => onChangeView('add')}
          title={$t('header.add')}
          type="button"
        >
          <Plus size={16} />
          {$t('header.add')}
        </button>
      {:else}
        <button
          class="flex items-center gap-2 rounded-xl border border-[var(--base-300)] px-3 py-2 text-sm text-[var(--base-content)]"
          onclick={() => onChangeView('list')}
          title={$t('header.back')}
          type="button"
        >
          <ChevronLeft size={16} />
          {$t('header.back')}
        </button>
      {/if}
    </div>
  </div>
</header>
