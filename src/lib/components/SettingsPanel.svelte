<script>
  import { t } from "../i18n";
  import { settings, updateSettings } from "../stores/settings";
  import { api } from "../api/skills";
  import { open } from "@tauri-apps/plugin-dialog";
  import { FolderOpen, Loader2, ChevronRight } from "@lucide/svelte";

  let checkingUpdate = $state(false);
  let updateMessage = $state("");

  // Backup state
  let backupFolder = $state("");
  let isBackingUp = $state(false);
  let lastBackupTime = $state("");
  let backupMessage = $state("");

  // Load backup folder and last backup time on mount
  $effect(() => {
    loadBackupFolder();
    loadLastBackupTime();
  });

  const loadBackupFolder = async () => {
    try {
      const folder = await api.getBackupFolder();
      if (folder) {
        backupFolder = folder;
      }
    } catch (error) {
      console.error("Failed to load backup folder:", error);
    }
  };

  const loadLastBackupTime = async () => {
    try {
      const time = await api.getLastBackupTime();
      if (time) {
        lastBackupTime = time;
      }
    } catch (error) {
      console.error("Failed to load last backup time:", error);
    }
  };

  const handleCheckUpdate = async () => {
    checkingUpdate = true;
    updateMessage = "";
    try {
      await new Promise((resolve) => setTimeout(resolve, 300));
      updateMessage = $t("settings.updatePlaceholder");
    } finally {
      checkingUpdate = false;
    }
  };

  const handleSelectBackupFolder = async () => {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: $t("settings.backup.selectFolder"),
      });
      if (selected) {
        const folder = Array.isArray(selected) ? selected[0] : selected;
        backupFolder = folder;
        await api.setBackupFolder(folder);
      }
    } catch (error) {
      console.error("Failed to select backup folder:", error);
    }
  };

  const handleBackup = async () => {
    if (!backupFolder) {
      await handleSelectBackupFolder();
      return;
    }

    isBackingUp = true;
    backupMessage = "";
    try {
      const result = await api.backupSkills(backupFolder);
      if (result.success) {
        lastBackupTime = result.backup_time || "";
      } else {
        backupMessage = result.message;
      }
    } catch (error) {
      backupMessage = error instanceof Error ? error.message : "Backup failed";
    } finally {
      isBackingUp = false;
    }
  };

  const handleOpenBackupFolder = async () => {
    if (!backupFolder) return;
    try {
      await api.openBackupFolder(backupFolder);
    } catch (error) {
      console.error("Failed to open backup folder:", error);
    }
  };
</script>

<section class="space-y-2">
  <!-- Language & Theme-->
  <div class="flex flex-col gap-4 rounded-2xl bg-[var(--base-200)] py-2.5 px-4">
    <div class="flex items-center justify-between">
      <span class="text-[15px] text-[var(--base-content)]"
        >{$t("settings.language")}</span
      >
      <div class="relative">
        <select
          class="appearance-none rounded-lg bg-[var(--base-300)] px-3 py-1.5 pr-9 text-[14px] text-[var(--base-content)] text-right focus:outline-none min-w-[120px] hover:bg-[var(--base-100)] transition-colors cursor-pointer"
          value={$settings.language}
          onchange={(event) =>
            updateSettings({ language: event.currentTarget.value })}
        >
          <option value="en">English</option>
          <option value="zh">中文</option>
        </select>
        <ChevronRight
          class="pointer-events-none absolute right-2.5 top-1/2 -translate-y-1/2 rotate-90 text-[var(--base-content-muted)]"
          size={14}
        />
      </div>
    </div>

    <div class="h-px bg-[var(--base-300)]"></div>

    <div class="flex items-center justify-between">
      <span class="text-[15px] text-[var(--base-content)]"
        >{$t("settings.theme")}</span
      >
      <div class="relative">
        <select
          class="appearance-none rounded-lg bg-[var(--base-300)] px-3 py-1.5 pr-9 text-[14px] text-[var(--base-content)] text-right focus:outline-none min-w-[120px] hover:bg-[var(--base-100)] transition-colors cursor-pointer"
          value={$settings.theme}
          onchange={(event) =>
            updateSettings({ theme: event.currentTarget.value })}
        >
          <option value="system">{$t("settings.theme.system")}</option>
          <option value="light">{$t("settings.theme.light")}</option>
          <option value="dark">{$t("settings.theme.dark")}</option>
        </select>
        <ChevronRight
          class="pointer-events-none absolute right-2.5 top-1/2 -translate-y-1/2 rotate-90 text-[var(--base-content-muted)]"
          size={14}
        />
      </div>
    </div>
  </div>

  <!-- Skill Sync Mode -->
  <div class="rounded-2xl bg-[var(--base-200)] py-2.5 px-4">
    <div class="flex items-center justify-between">
      <span class="text-[15px] text-[var(--base-content)]"
        >{$t("settings.syncMode")}</span
      >
      <div class="relative">
        <select
          class="appearance-none rounded-lg bg-[var(--base-300)] px-3 py-1.5 pr-9 text-[14px] text-[var(--base-content)] text-right focus:outline-none min-w-[120px] hover:bg-[var(--base-100)] transition-colors cursor-pointer"
          value={$settings.syncMode}
          onchange={(event) =>
            updateSettings({ syncMode: event.currentTarget.value })}
        >
          <option value="symlink">{$t("settings.syncMode.symlink")}</option>
          <option value="copy">{$t("settings.syncMode.copy")}</option>
        </select>
        <ChevronRight
          class="pointer-events-none absolute right-2.5 top-1/2 -translate-y-1/2 rotate-90 text-[var(--base-content-muted)]"
          size={14}
        />
      </div>
    </div>
  </div>

  <!-- Backup -->
  <div class="rounded-2xl bg-[var(--base-200)] py-2.5 px-4">
    <div class="flex items-center justify-between">
      <span class="text-[15px] text-[var(--base-content)]"
        >{$t("settings.backup")}</span
      >
      <div class="flex items-center gap-3">
        {#if backupFolder}
          <span class="text-xs text-[var(--base-content-muted)]">
            {#if lastBackupTime}
              {$t("settings.backup.lastBackup", { time: lastBackupTime })}
            {:else}
              {$t("settings.backup.noBackupYet")}
            {/if}
          </span>
        {/if}
        <button
          class="flex items-center gap-1.5 rounded-lg bg-[var(--primary)] px-3 py-1.5 text-[13px] text-[var(--primary-content)] hover:opacity-90 disabled:opacity-50"
          onclick={handleBackup}
          disabled={isBackingUp}
          type="button"
        >
          {#if isBackingUp}
            <Loader2 size={13} class="animate-spin" />
          {/if}
          {isBackingUp
            ? $t("settings.backup.backingUp")
            : $t("settings.backup.backupNow")}
        </button>
      </div>
    </div>
    {#if backupFolder}
      <div class="mt-1.5 flex items-center gap-2">
        <span class="text-xs text-[var(--base-content-muted)] break-all"
          >{backupFolder}</span
        >
        <button
          class="rounded p-1 text-[var(--base-content-muted)] hover:bg-[var(--base-300)] hover:text-[var(--base-content)]"
          onclick={handleOpenBackupFolder}
          title={$t("settings.backup.openFolder")}
          type="button"
        >
          <FolderOpen size={13} />
        </button>
      </div>
    {/if}
    {#if backupMessage}
      <span class="mt-1.5 block text-xs text-red-500">
        {backupMessage}
      </span>
    {/if}
  </div>

  <!-- Updates -->
  <div class="rounded-2xl bg-[var(--base-200)] py-2.5 px-4">
    <div class="flex items-center justify-between">
      <span class="text-[15px] text-[var(--base-content)]"
        >{$t("settings.about")}</span
      >
      <button
        class="rounded-lg bg-[var(--primary)] px-3 py-1.5 text-[13px] text-[var(--primary-content)] hover:opacity-90 disabled:opacity-50"
        onclick={handleCheckUpdate}
        disabled={checkingUpdate}
        type="button"
      >
        {checkingUpdate
          ? $t("settings.checkingUpdate")
          : $t("settings.checkUpdateBtn")}
      </button>
    </div>
    {#if updateMessage}
      <span class="mt-1.5 block text-xs text-[var(--base-content-muted)]"
        >{updateMessage}</span
      >
    {/if}
  </div>
</section>
