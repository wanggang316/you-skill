<script>
  import { t } from '../i18n'
  import { settings, updateSettings } from '../stores/settings'
  import { api } from '../api/skills'
  import { open } from '@tauri-apps/plugin-dialog'
  import { Folder, FolderOpen, Loader2 } from '@lucide/svelte'

  let checkingUpdate = $state(false)
  let updateMessage = $state('')

  // Backup state
  let backupFolder = $state('')
  let isBackingUp = $state(false)
  let lastBackupTime = $state('')
  let backupMessage = $state('')

  // Load backup folder on mount
  $effect(() => {
    loadBackupFolder()
  })

  const loadBackupFolder = async () => {
    try {
      const folder = await api.getBackupFolder()
      if (folder) {
        backupFolder = folder
      }
    } catch (error) {
      console.error('Failed to load backup folder:', error)
    }
  }

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

  const handleSelectBackupFolder = async () => {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: $t('settings.backup.selectFolder')
      })
      if (selected) {
        const folder = Array.isArray(selected) ? selected[0] : selected
        backupFolder = folder
        await api.setBackupFolder(folder)
      }
    } catch (error) {
      console.error('Failed to select backup folder:', error)
    }
  }

  const handleBackup = async () => {
    if (!backupFolder) {
      backupMessage = $t('settings.backup.noFolder')
      return
    }

    isBackingUp = true
    backupMessage = ''
    try {
      const result = await api.backupSkills(backupFolder)
      if (result.success) {
        lastBackupTime = result.backup_time || ''
        // 成功时不显示文案，直接更新时间
      } else {
        backupMessage = result.message
      }
    } catch (error) {
      backupMessage = error instanceof Error ? error.message : 'Backup failed'
    } finally {
      isBackingUp = false
    }
  }

  const handleOpenBackupFolder = async () => {
    if (!backupFolder) return
    try {
      await api.openBackupFolder(backupFolder)
    } catch (error) {
      console.error('Failed to open backup folder:', error)
    }
  }
</script>

<section class="space-y-6">
  <!-- Language -->
  <div class="rounded-2xl border border-[var(--base-300)] bg-[var(--base-100)] p-5">
    <label class="space-y-3 text-sm text-[var(--base-content)]">
      <span class="text-base font-medium text-[var(--base-content)]">{$t('settings.language')}</span>
      <select
        class="w-full rounded-xl border border-[var(--base-300)] bg-[var(--base-100)] px-3 py-2"
        value={$settings.language}
        onchange={(event) => updateSettings({ language: event.currentTarget.value })}
      >
        <option value="en">English</option>
        <option value="zh">中文</option>
      </select>
    </label>
  </div>

  <!-- Theme -->
  <div class="rounded-2xl border border-[var(--base-300)] bg-[var(--base-100)] p-5">
    <label class="space-y-3 text-sm text-[var(--base-content)]">
      <span class="text-base font-medium text-[var(--base-content)]">{$t('settings.theme')}</span>
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
  </div>

  <!-- Skill Sync Mode -->
  <div class="rounded-2xl border border-[var(--base-300)] bg-[var(--base-100)] p-5">
    <label class="space-y-3 text-sm text-[var(--base-content)]">
      <span class="text-base font-medium text-[var(--base-content)]">{$t('settings.syncMode')}</span>
      <select
        class="w-full rounded-xl border border-[var(--base-300)] bg-[var(--base-100)] px-3 py-2"
        value={$settings.syncMode}
        onchange={(event) => updateSettings({ syncMode: event.currentTarget.value })}
      >
        <option value="symlink">{$t('settings.syncMode.symlink')}</option>
        <option value="copy">{$t('settings.syncMode.copy')}</option>
      </select>
    </label>
  </div>

  <!-- Backup -->
  <div class="rounded-2xl border border-[var(--base-300)] bg-[var(--base-100)] p-5">
    <div class="space-y-4">
      <div class="flex items-center justify-between">
        <p class="text-base font-medium text-[var(--base-content)]">{$t('settings.backup')}</p>
        {#if lastBackupTime}
          <span class="text-xs text-[var(--base-content-muted)]">
            {$t('settings.backup.lastBackup', { time: lastBackupTime })}
          </span>
        {:else}
          <span class="text-xs text-[var(--base-content-muted)]">
            {$t('settings.backup.noBackupYet')}
          </span>
        {/if}
      </div>

      <!-- Backup Folder Path with Open Icon -->
      {#if backupFolder}
        <div class="flex items-center gap-2">
          <p class="flex-1 text-xs text-[var(--base-content-muted)] break-all">
            {backupFolder}
          </p>
          <button
            class="flex-shrink-0 rounded-lg p-1 text-[var(--base-content-muted)] hover:bg-[var(--base-200)] hover:text-[var(--base-content)]"
            onclick={handleOpenBackupFolder}
            title={$t('settings.backup.openFolder')}
            type="button"
          >
            <FolderOpen size={14} />
          </button>
        </div>
      {:else}
        <p class="text-xs text-[var(--base-content-muted)]">
          {$t('settings.backup.noFolder')}
        </p>
      {/if}

      <!-- Backup Button -->
      <div class="flex items-center gap-2">
        <button
          class="flex items-center gap-2 rounded-xl bg-[var(--primary)] px-4 py-2 text-sm text-[var(--primary-content)] hover:opacity-90 disabled:opacity-50"
          onclick={handleBackup}
          disabled={isBackingUp || !backupFolder}
          type="button"
        >
          {#if isBackingUp}
            <Loader2 size={16} class="animate-spin" />
            {$t('settings.backup.backingUp')}
          {:else}
            {$t('settings.backup.backupNow')}
          {/if}
        </button>
        <button
          class="flex items-center gap-2 rounded-xl border border-[var(--base-300)] bg-[var(--base-200)] px-4 py-2 text-sm text-[var(--base-content)] hover:bg-[var(--base-300)]"
          onclick={handleSelectBackupFolder}
          type="button"
        >
          <Folder size={16} />
          {$t('settings.backup.selectFolder')}
        </button>
      </div>

      <!-- Error Message (only show errors, not success) -->
      {#if backupMessage}
        <span class="block text-sm text-[var(--base-content-muted)]">
          {backupMessage}
        </span>
      {/if}
    </div>
  </div>

  <!-- Updates -->
  <div class="rounded-2xl border border-[var(--base-300)] bg-[var(--base-100)] p-5 space-y-3">
    <p class="text-base font-medium text-[var(--base-content)]">{$t('settings.about')}</p>
    <button
      class="rounded-xl bg-[var(--primary)] px-4 py-2 text-sm text-[var(--primary-content)] hover:opacity-90"
      onclick={handleCheckUpdate}
      disabled={checkingUpdate}
      type="button"
    >
      {checkingUpdate ? $t('settings.checkingUpdate') : $t('settings.checkUpdate')}
    </button>
    {#if updateMessage}
      <span class="mt-3 block text-sm text-[var(--base-content-muted)]">
        {updateMessage}
      </span>
    {/if}
  </div>
</section>
