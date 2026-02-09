<script>
  import { t } from "../i18n";
  import { settings, updateSettings } from "../stores/settings";
  import {
  getBackupFolder,
  setBackupFolder,
  openBackupFolder,
  backupSkills,
  getLastBackupTime,
} from "../api";
  import { open } from "@tauri-apps/plugin-dialog";
  import { FolderOpen, Loader2, ChevronRight, Download } from "@lucide/svelte";
  import { check } from "@tauri-apps/plugin-updater";
  import { relaunch } from "@tauri-apps/plugin-process";
  import { getVersion } from "@tauri-apps/api/app";

  let checkingUpdate = $state(false);
  let updateMessage = $state("");
  let currentVersion = $state("");
  let hasUpdate = $state(false);
  let latestVersion = $state("");
  let updateInstance = $state(null);
  let isInstalling = $state(false);
  let downloadProgress = $state(0);

  // Backup state
  let backupFolder = $state("");
  let isBackingUp = $state(false);
  let lastBackupTime = $state("");
  let backupMessage = $state("");

  // Load backup folder, last backup time, and version on mount
  $effect(() => {
    loadBackupFolder();
    loadLastBackupTime();
    loadVersion();
  });

  const loadVersion = async () => {
    try {
      currentVersion = await getVersion();
    } catch (error) {
      console.error("Failed to load app version:", error);
    }
  };

  const loadBackupFolder = async () => {
    try {
      const folder = await getBackupFolder();
      if (folder) {
        backupFolder = folder;
      }
    } catch (error) {
      console.error("Failed to load backup folder:", error);
    }
  };

  const loadLastBackupTime = async () => {
    try {
      const time = await getLastBackupTime();
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
    hasUpdate = false;
    downloadProgress = 0;
    try {
      const update = await check();
      if (update) {
        hasUpdate = true;
        latestVersion = update.version;
        updateInstance = update;
      } else {
        updateMessage = $t("settings.noUpdateAvailable");
      }
    } catch (error) {
      updateMessage = $t("settings.updateCheckFailed");
      console.error("Failed to check for update:", error);
    } finally {
      checkingUpdate = false;
    }
  };

  const handleInstallUpdate = async () => {
    if (!updateInstance) return;
    isInstalling = true;
    downloadProgress = 0;
    try {
      await updateInstance.downloadAndInstall((event) => {
        switch (event.event) {
          case "Started":
            downloadProgress = 0;
            break;
          case "Progress":
            downloadProgress = event.data.chunkLength;
            break;
          case "Finished":
            downloadProgress = 100;
            break;
        }
      });
      // 安装完成后重启应用
      await relaunch();
    } catch (error) {
      updateMessage = error instanceof Error ? error.message : $t("settings.updateInstallFailed");
      isInstalling = false;
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
        await setBackupFolder(folder);
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
      const result = await backupSkills(backupFolder);
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
      await openBackupFolder(backupFolder);
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
          class="flex items-center rounded-lg bg-[var(--primary)] px-3 py-1.5 text-[13px] text-[var(--primary-content)] hover:bg-[var(--primary-hover)] disabled:opacity-50"
          onclick={handleBackup}
          disabled={isBackingUp}
          type="button"
        >
          {#if isBackingUp}
            <Loader2 size={13} class="animate-spin mr-1.5" />
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
      <div class="flex flex-col">
        <span class="text-[15px] text-[var(--base-content)]"
          >{$t("settings.about")}</span
        >
        {#if currentVersion}
          <span class="text-xs text-[var(--base-content-muted)]">
            {$t("settings.currentVersion", { version: currentVersion })}
          </span>
        {/if}
      </div>
      {#if hasUpdate}
        <div class="flex items-center gap-2">
          <span class="text-xs text-[var(--success)] font-medium">
            {$t("settings.newVersionAvailable", { version: latestVersion })}
          </span>
          <button
            class="flex items-center rounded-lg bg-[var(--success)] px-3 py-1.5 text-[13px] text-[var(--success-content)] hover:bg-[var(--primary-hover)] disabled:opacity-50"
            onclick={handleInstallUpdate}
            disabled={isInstalling}
            type="button"
          >
            {#if isInstalling}
              <Loader2 size={13} class="animate-spin mr-1.5" />
            {:else}
              <Download size={13} class="mr-1.5" />
            {/if}
            {isInstalling
              ? $t("settings.installingUpdate")
              : $t("settings.installUpdate")}
          </button>
        </div>
      {:else}
        <button
          class="rounded-lg bg-[var(--primary)] px-3 py-1.5 text-[13px] text-[var(--primary-content)] hover:bg-[var(--primary-hover)] disabled:opacity-50"
          onclick={handleCheckUpdate}
          disabled={checkingUpdate}
          type="button"
        >
          {checkingUpdate
            ? $t("settings.checkingUpdate")
            : $t("settings.checkUpdateBtn")}
        </button>
      {/if}
    </div>
    {#if updateMessage}
      <span class="mt-1.5 block text-xs text-[var(--base-content-muted)]"
        >{updateMessage}</span
      >
    {/if}
  </div>
</section>
