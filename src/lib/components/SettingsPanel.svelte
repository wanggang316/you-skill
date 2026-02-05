<script>
  import { t } from '../i18n'
  import { settings, updateSettings } from '../stores/settings'

  let checkingUpdate = $state(false)
  let updateMessage = $state('')

  const handleCheckUpdate = async () => {
    checkingUpdate = true
    updateMessage = ''
    try {
      await new Promise((resolve) => setTimeout(resolve, 300))
      updateMessage = $t('settings.updatePlaceholder')
    } finally {
      checkingUpdate = false
    }
  }
</script>

<section class="space-y-6">
  <div class="rounded-2xl border border-[var(--base-300)] bg-[var(--base-100)] p-5">
    <div class="flex items-center justify-between">
      <div>
        <p class="text-sm font-semibold">{$t('settings.general')}</p>
        <p class="text-xs text-[var(--base-content-muted)]">
          {$t('settings.general.subtitle')}
        </p>
      </div>
    </div>
    <div class="mt-4 grid gap-4 md:grid-cols-3">
      <label class="space-y-2 text-sm text-[var(--base-content)]">
        <span class="text-xs text-[var(--base-content-muted)]">
          {$t('settings.language')}
        </span>
        <select
          class="w-full rounded-xl border border-[var(--base-300)] bg-[var(--base-100)] px-3 py-2"
          value={$settings.language}
          onchange={(event) =>
            updateSettings({ language: event.currentTarget.value })}
        >
          <option value="en">English</option>
          <option value="zh">中文</option>
        </select>
      </label>
      <label class="space-y-2 text-sm text-[var(--base-content)]">
        <span class="text-xs text-[var(--base-content-muted)]">
          {$t('settings.theme')}
        </span>
        <select
          class="w-full rounded-xl border border-[var(--base-300)] bg-[var(--base-100)] px-3 py-2"
          value={$settings.theme}
          onchange={(event) => updateSettings({ theme: event.currentTarget.value })}
        >
          <option value="system">{$t('settings.theme.system')}</option>
          <option value="light">{$t('settings.theme.light')}</option>
          <option value="dark">{$t('settings.theme.dark')}</option>
        </select>
      </label>
      <label class="space-y-2 text-sm text-[var(--base-content)]">
        <span class="text-xs text-[var(--base-content-muted)]">
          {$t('settings.syncMode')}
        </span>
        <select
          class="w-full rounded-xl border border-[var(--base-300)] bg-[var(--base-100)] px-3 py-2"
          value={$settings.syncMode}
          onchange={(event) =>
            updateSettings({ syncMode: event.currentTarget.value })}
        >
          <option value="symlink">{$t('settings.syncMode.symlink')}</option>
          <option value="copy">{$t('settings.syncMode.copy')}</option>
        </select>
      </label>
    </div>
  </div>

  <div class="rounded-2xl border border-[var(--base-300)] bg-[var(--base-100)] p-5">
    <button
      class="rounded-xl bg-[var(--primary)] px-4 py-2 text-sm text-[var(--primary-content)] hover:opacity-90"
      onclick={handleCheckUpdate}
      disabled={checkingUpdate}
      type="button"
    >
      {checkingUpdate
        ? $t('settings.checkingUpdate')
        : $t('settings.checkUpdate')}
    </button>
    {#if updateMessage}
      <span class="mt-3 block text-sm text-[var(--base-content-muted)]">
        {updateMessage}
      </span>
    {/if}
  </div>
</section>
