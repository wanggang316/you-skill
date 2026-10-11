<script lang="ts">
  import { onMount } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { FolderOpen } from "@lucide/svelte";
  import { backupSkills, getSettings, openBackupFolder, setBackupFolder } from "$lib/api";
  import IconButton from "$lib/components/ui/IconButton.svelte";
  import PrimaryActionButton from "$lib/components/ui/PrimaryActionButton.svelte";
  import { t } from "$lib/i18n";
  import SettingsPane from "./SettingsPane.svelte";
  import SettingsRow from "./SettingsRow.svelte";

  let backupFolder = $state("");
  let lastBackupTime = $state("");
  let isBackingUp = $state(false);
  let message = $state("");

  onMount(async () => {
    try {
      const data = await getSettings();
      backupFolder = data.backup_folder ?? "";
      lastBackupTime = data.last_backup_time ?? "";
    } catch (error) {
      console.error("Failed to load backup state from settings:", error);
    }
  });

  async function selectFolder(): Promise<boolean> {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: $t("settings.backup.selectFolder"),
      });
      if (!selected) return false;
      const folder = Array.isArray(selected) ? selected[0] : selected;
      backupFolder = folder;
      await setBackupFolder(folder);
      return true;
    } catch (error) {
      console.error("Failed to select backup folder:", error);
      return false;
    }
  }

  async function handleBackup() {
    if (!backupFolder && !(await selectFolder())) return;
    isBackingUp = true;
    message = "";
    try {
      const result = await backupSkills(backupFolder);
      if (result.success) {
        lastBackupTime = result.backup_time || "";
      } else {
        message = result.message;
      }
    } catch (error) {
      message = error instanceof Error ? error.message : $t("settings.backup.failed");
    } finally {
      isBackingUp = false;
    }
  }

  async function handleOpenFolder() {
    if (!backupFolder) return;
    try {
      await openBackupFolder(backupFolder);
    } catch (error) {
      console.error("Failed to open backup folder:", error);
    }
  }
</script>

<SettingsPane title={$t("settings.section.backup")} description={$t("settings.backup.description")}>
  <div class="settings-card">
    <SettingsRow
      label={$t("settings.backup.folder")}
      description={backupFolder || $t("settings.notSet")}
    >
      {#if backupFolder}
        <IconButton
          variant="ghost"
          onclick={handleOpenFolder}
          title={$t("settings.backup.openFolder")}
          class="h-8 w-8 p-0"
        >
          <FolderOpen size={15} />
        </IconButton>
      {/if}
      <button class="settings-button" type="button" onclick={selectFolder}>
        {backupFolder ? $t("settings.backup.changeFolder") : $t("settings.backup.chooseFolder")}
      </button>
    </SettingsRow>
    <SettingsRow
      label={$t("settings.backup.status")}
      description={lastBackupTime || $t("settings.backup.noBackupYet")}
    >
      <PrimaryActionButton
        className="h-8 rounded-lg px-3 py-0 text-[13px]"
        onclick={handleBackup}
        loading={isBackingUp}
        loadingText={$t("settings.backup.backingUp")}
      >
        {$t("settings.backup.backupNow")}
      </PrimaryActionButton>
    </SettingsRow>
  </div>

  {#if message}
    <p class="text-error px-1 text-xs break-all">{message}</p>
  {/if}
</SettingsPane>
