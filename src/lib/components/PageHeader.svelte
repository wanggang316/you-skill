<script>
  import { ChevronLeft, Plus, Settings } from '@lucide/svelte'
  import IconButton from './IconButton.svelte'
  import { t } from '../i18n'

  const { currentView, activeTab, onChangeView, onChangeTab } = $props()
</script>

<header class="border-b border-[var(--base-300)] bg-[var(--base-100)]">
  <div class="mx-auto flex max-w-6xl items-center justify-between px-6 py-4">
    {#if currentView === 'list'}
      <div class="flex items-center gap-2">
        <span class="text-2xl">⚡️</span>
        <p class="text-lg font-semibold italic">Skill Kit</p>
      </div>
    {/if}
    <div class="flex flex-1 items-center justify-between {currentView === 'list' ? 'pl-6' : ''} text-sm">
      {#if currentView === 'list'}
        <div class="flex items-center gap-3">
          <div class="flex items-center gap-2 rounded-full bg-[var(--base-200)] p-1">
            <button
              class={`rounded-full px-4 py-2 transition hover:text-[var(--base-content)] ${activeTab === 'local' ? 'bg-[var(--base-100)] text-[var(--base-content)] shadow-sm' : 'text-[var(--base-content-muted)]'}`}
              onclick={() => onChangeTab('local')}
              type="button"
            >
              {$t('header.localTab')}
            </button>
            <button
              class={`rounded-full px-4 py-2 transition hover:text-[var(--base-content)] ${activeTab === 'remote' ? 'bg-[var(--base-100)] text-[var(--base-content)] shadow-sm' : 'text-[var(--base-content-muted)]'}`}
              onclick={() => onChangeTab('remote')}
              type="button"
            >
              {$t('header.remoteTab')}
            </button>
          </div>
          <IconButton
            variant="primary"
            onclick={() => onChangeView('add')}
            title={$t('header.add')}
            ariaLabel={$t('header.add')}
          >
            <Plus size={16} />
          </IconButton>
        </div>
        <IconButton
          variant="outline"
          onclick={() => onChangeView('settings')}
          title={$t('header.settings')}
          ariaLabel={$t('header.settings')}
        >
          <Settings size={16} />
        </IconButton>
      {:else}
        <button
          class="flex items-center gap-2 rounded-xl border border-[var(--base-300)] px-3 py-2 text-sm text-[var(--base-content)] transition hover:bg-[var(--base-200)]"
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
