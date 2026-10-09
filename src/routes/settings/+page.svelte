<script lang="ts">
  import { goto } from "$app/navigation";
  import { page } from "$app/stores";
  import { t } from "$lib/i18n";
  import { settings, updateSettings } from "$lib/stores/settings";
  import { get } from "svelte/store";
  import {
    getSettings,
    setBackupFolder,
    openBackupFolder,
    backupSkills,
    getGithubAuthStatus,
    getCliStatus,
    installCli,
    uninstallCli,
    type CliStatus,
    type GithubAuthStatus,
  } from "$lib/api";
  import TranslateSettingsModal, {
    type TranslateSettingsPayload,
  } from "$lib/components/TranslateSettingsModal.svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { FolderOpen, Loader2, ChevronRight, Download } from "@lucide/svelte";
  import { check, type Update } from "@tauri-apps/plugin-updater";
  import { relaunch } from "@tauri-apps/plugin-process";
  import { getVersion } from "@tauri-apps/api/app";
  import type { DownloadEvent } from "@tauri-apps/plugin-updater";

  let checkingUpdate = $state(false);
  let updateMessage = $state("");
  let currentVersion = $state("");
  let hasUpdate = $state(false);
  let latestVersion = $state("");
  let updateInstance = $state<Update | null>(null);
  let isInstalling = $state(false);
  let downloadProgress = $state(0);
  let downloadTotalBytes = $state(0);
  let downloadedBytes = $state(0);
  // Backup state
  let backupFolder = $state("");
  let isBackingUp = $state(false);
  let lastBackupTime = $state("");
  let backupMessage = $state("");

  // Agent apps state
  let installedAgentAppsCount = $state(0);
  let translateSettingsOpen = $state(false);
  let savingTranslateSettings = $state(false);
  const translationSummary = $derived.by(() => {
    const targetLanguage = ($settings.translate_target_language || "").trim();
    const model = ($settings.translate_model || "").trim();
    const parts: string[] = [];
    if (targetLanguage) {
      parts.push($t("settings.translation.targetLanguageValue", { language: targetLanguage }));
    }
    if (model) {
      parts.push($t("settings.translation.modelValue", { model }));
    }
    return parts.join(" · ");
  });

  // Load installed agent apps count on mount
  $effect(() => {
    loadAgentAppsCount();
  });

  let githubStatus = $state<GithubAuthStatus | null>(null);
  const githubSummary = $derived.by(() => {
    if (!githubStatus) return "";
    if (!githubStatus.gh_installed) return $t("settings.github.summaryNoGh");
    if (githubStatus.selected_login) {
      return $t("settings.github.summarySignedIn", { login: githubStatus.selected_login });
    }
    return $t("settings.github.summaryNotSignedIn");
  });

  $effect(() => {
    loadGithubStatus();
    loadCliStatus();
  });

  // Command line tool state
  let cliStatus = $state<CliStatus | null>(null);
  let cliBusy = $state(false);
  let cliMessage = $state("");
  let cliMessageIsError = $state(false);
  const cliSummary = $derived.by(() => {
    if (!cliStatus) return "";
    if (!cliStatus.bundled_path) return $t("settings.cli.notBundled");
    if (!cliStatus.installed) return $t("settings.cli.notInstalled");
    if (cliStatus.current) return $t("settings.cli.installedAt", { path: cliStatus.install_path });
    return $t("settings.cli.outdated", { path: cliStatus.install_path });
  });

  const loadCliStatus = async () => {
    try {
      cliStatus = await getCliStatus();
    } catch (error) {
      console.error("Failed to load command line tool status:", error);
    }
  };

  const handleInstallCli = async () => {
    cliBusy = true;
    cliMessage = "";
    cliMessageIsError = false;
    try {
      cliStatus = await installCli();
      cliMessage = $t("settings.cli.newTerminal");
    } catch (error) {
      cliMessage = getErrorMessage(error, $t("settings.cli.installFailed"));
      cliMessageIsError = true;
      console.error("Failed to install the command line tool:", error);
    } finally {
      cliBusy = false;
    }
  };

  const handleRemoveCli = async () => {
    cliBusy = true;
    cliMessage = "";
    cliMessageIsError = false;
    try {
      cliStatus = await uninstallCli();
    } catch (error) {
      cliMessage = getErrorMessage(error, $t("settings.cli.removeFailed"));
      cliMessageIsError = true;
      console.error("Failed to remove the command line tool:", error);
    } finally {
      cliBusy = false;
    }
  };

  const loadGithubStatus = async () => {
    try {
      githubStatus = await getGithubAuthStatus();
    } catch (error) {
      console.error("Failed to load GitHub status:", error);
    }
  };

  const loadAgentAppsCount = async () => {
    try {
      const { listLocalAgentApps } = await import("$lib/api");
      const apps = await listLocalAgentApps();
      installedAgentAppsCount = apps.length;
    } catch (error) {
      console.error("Failed to load agent apps count:", error);
    }
  };

  // Load settings-derived backup state and version on mount
  $effect(() => {
    loadBackupState();
    loadVersion();
  });

  const loadVersion = async () => {
    try {
      currentVersion = await getVersion();
    } catch (error) {
      console.error("Failed to load app version:", error);
    }
  };

  const loadBackupState = async () => {
    try {
      const data = await getSettings();
      const folder = data.backup_folder;
      if (folder) {
        backupFolder = folder;
      }
      const time = data.last_backup_time;
      if (time) {
        lastBackupTime = time;
      }
    } catch (error) {
      console.error("Failed to load backup state from settings:", error);
    }
  };

  const getErrorMessage = (error: unknown, fallback: string): string => {
    if (error instanceof Error && error.message) return error.message;
    if (typeof error === "string" && error.trim()) return error;
    try {
      const text = JSON.stringify(error);
      return text && text !== "{}" ? text : fallback;
    } catch {
      return fallback;
    }
  };

  const handleCheckUpdate = async () => {
    checkingUpdate = true;
    updateMessage = "";
    hasUpdate = false;
    downloadProgress = 0;
    downloadTotalBytes = 0;
    downloadedBytes = 0;
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
    downloadTotalBytes = 0;
    downloadedBytes = 0;
    try {
      await updateInstance.downloadAndInstall((event: DownloadEvent) => {
        switch (event.event) {
          case "Started":
            downloadTotalBytes = event.data.contentLength ?? 0;
            downloadedBytes = 0;
            downloadProgress = 0;
            break;
          case "Progress":
            downloadedBytes += event.data.chunkLength;
            if (downloadTotalBytes > 0) {
              downloadProgress = Math.min(
                100,
                Math.round((downloadedBytes / downloadTotalBytes) * 100)
              );
            }
            break;
          case "Finished":
            downloadProgress = 100;
            break;
        }
      });
      // 安装完成后重启应用
      await relaunch();
    } catch (error) {
      updateMessage = getErrorMessage(error, $t("settings.updateInstallFailed"));
      console.error("Failed to install update:", error);
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

  const navigateToAgentApps = () => {
    const currentUrl = get(page).url;
    const returnTo = encodeURIComponent(`${currentUrl.pathname}${currentUrl.search}`);
    goto(`/agent-apps?returnTo=${returnTo}`);
  };

  const handleSaveTranslateSettings = async (payload: TranslateSettingsPayload) => {
    savingTranslateSettings = true;
    try {
      await updateSettings({
        openrouter_api_key: payload.apiKey || null,
        translate_target_language: payload.targetLanguage.trim(),
        translate_model: payload.model.trim(),
        translate_display_mode: payload.displayMode,
        translate_text_style: payload.textStyle,
      });
      translateSettingsOpen = false;
    } finally {
      savingTranslateSettings = false;
    }
  };
</script>

<section class="flex h-full min-h-0 min-w-0 flex-col overflow-hidden">
  <header
    class="border-base-300 flex h-12 flex-none items-center border-b px-7"
    data-window-drag-region
  >
    <h1 class="text-base-content text-[1.05rem] font-semibold tracking-[-0.02em]">
      {$t("header.settings")}
    </h1>
  </header>

  <main class="min-h-0 flex-1 overflow-y-auto">
    <div class="mx-auto max-w-4xl px-7 py-6">
      <section class="space-y-4">
        <!-- Language & Theme-->
        <div class="bg-base-200 flex flex-col gap-4 rounded-2xl px-4 py-2.5">
          <div class="flex items-center justify-between">
            <span class="text-base-content text-[15px]">{$t("settings.language")}</span>
            <div class="relative">
              <select
                class="bg-base-300 text-base-content hover:bg-base-100 min-w-[120px] cursor-pointer appearance-none rounded-lg px-3 py-1.5 pr-9 text-right text-[14px] transition-colors focus:outline-none"
                value={$settings.language}
                onchange={(event) =>
                  updateSettings({
                    language: event.currentTarget
                      .value as import("$lib/api/settings").AppSettings["language"],
                  })}
              >
                <option value="en">English</option>
                <option value="zh">中文</option>
              </select>
              <ChevronRight
                class="text-base-content-muted pointer-events-none absolute top-1/2 right-2.5 -translate-y-1/2 rotate-90"
                size={14}
              />
            </div>
          </div>

          <div class="bg-base-300 h-px"></div>

          <div class="flex items-center justify-between">
            <span class="text-base-content text-[15px]">{$t("settings.theme")}</span>
            <div class="relative">
              <select
                class="bg-base-300 text-base-content hover:bg-base-100 min-w-[120px] cursor-pointer appearance-none rounded-lg px-3 py-1.5 pr-9 text-right text-[14px] transition-colors focus:outline-none"
                value={$settings.theme}
                onchange={(event) =>
                  updateSettings({
                    theme: event.currentTarget
                      .value as import("$lib/api/settings").AppSettings["theme"],
                  })}
              >
                <option value="system">{$t("settings.theme.system")}</option>
                <option value="light">{$t("settings.theme.light")}</option>
                <option value="dark">{$t("settings.theme.dark")}</option>
              </select>
              <ChevronRight
                class="text-base-content-muted pointer-events-none absolute top-1/2 right-2.5 -translate-y-1/2 rotate-90"
                size={14}
              />
            </div>
          </div>
        </div>

        <!-- Default install mode -->
        <div class="bg-base-200 rounded-2xl px-4 py-2.5">
          <div class="flex items-center justify-between">
            <span class="text-base-content text-[15px]">{$t("settings.installMode")}</span>
            <div class="relative">
              <select
                class="bg-base-300 text-base-content hover:bg-base-100 min-w-[120px] cursor-pointer appearance-none rounded-lg px-3 py-1.5 pr-9 text-right text-[14px] transition-colors focus:outline-none"
                value={$settings.sync_mode}
                onchange={(event) =>
                  updateSettings({
                    sync_mode: event.currentTarget
                      .value as import("$lib/api/settings").AppSettings["sync_mode"],
                  })}
              >
                <option value="copy">{$t("settings.syncMode.copy")}</option>
                <option value="symlink">{$t("settings.syncMode.symlink")}</option>
              </select>
              <ChevronRight
                class="text-base-content-muted pointer-events-none absolute top-1/2 right-2.5 -translate-y-1/2 rotate-90"
                size={14}
              />
            </div>
          </div>
        </div>

        <!-- Agent Apps -->
        <div class="bg-base-200 rounded-2xl px-4 py-2.5">
          <div class="flex items-center justify-between">
            <span class="text-base-content text-[15px]">{$t("agentApps.title")}</span>
            <button
              class="bg-primary text-primary-content hover:bg-primary-hover rounded-lg px-3 py-1.5 text-[13px]"
              onclick={navigateToAgentApps}
              type="button"
            >
              {$t("agentApps.configButton")}
            </button>
          </div>
        </div>

        <!-- GitHub -->
        <div class="bg-base-200 rounded-2xl px-4 py-2.5">
          <div class="flex items-center justify-between">
            <div class="flex flex-col">
              <span class="text-base-content text-[15px]">{$t("settings.github.title")}</span>
              {#if githubSummary}
                <span class="text-base-content-muted text-xs">{githubSummary}</span>
              {/if}
            </div>
            <button
              class="bg-primary text-primary-content hover:bg-primary-hover rounded-lg px-3 py-1.5 text-[13px]"
              onclick={() => goto("/settings/github")}
              type="button"
            >
              {$t("agentApps.configButton")}
            </button>
          </div>
        </div>

        <!-- Translation -->
        <div class="bg-base-200 rounded-2xl px-4 py-2.5">
          <div class="flex items-center justify-between">
            <div class="flex flex-col">
              <span class="text-base-content text-[15px]">{$t("settings.translation.title")}</span>
              {#if translationSummary}
                <span class="text-base-content-muted text-xs">{translationSummary}</span>
              {/if}
            </div>
            <button
              class="bg-primary text-primary-content hover:bg-primary-hover rounded-lg px-3 py-1.5 text-[13px]"
              onclick={() => {
                translateSettingsOpen = true;
              }}
              type="button"
            >
              {$t("agentApps.configButton")}
            </button>
          </div>
        </div>

        <!-- Backup -->
        <div class="bg-base-200 rounded-2xl px-4 py-2.5">
          <div class="flex items-center justify-between">
            <span class="text-base-content text-[15px]">{$t("settings.backup")}</span>
            <div class="flex items-center gap-3">
              {#if backupFolder}
                <span class="text-base-content-muted text-xs">
                  {#if lastBackupTime}
                    {$t("settings.backup.lastBackup", { time: lastBackupTime })}
                  {:else}
                    {$t("settings.backup.noBackupYet")}
                  {/if}
                </span>
              {/if}
              <button
                class="bg-primary text-primary-content hover:bg-primary-hover flex items-center rounded-lg px-3 py-1.5 text-[13px] disabled:opacity-50"
                onclick={handleBackup}
                disabled={isBackingUp}
                type="button"
              >
                {#if isBackingUp}
                  <Loader2 size={13} class="mr-1.5 animate-spin" />
                {/if}
                {isBackingUp ? $t("settings.backup.backingUp") : $t("settings.backup.backupNow")}
              </button>
            </div>
          </div>
          {#if backupFolder}
            <div class="mt-1.5 flex items-center gap-2">
              <span class="text-base-content-muted text-xs break-all">{backupFolder}</span>
              <button
                class="text-base-content-muted hover:bg-base-300 hover:text-base-content rounded p-1"
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

        <!-- Command line tool -->
        <div class="bg-base-200 rounded-2xl px-4 py-2.5">
          <div class="flex items-center justify-between">
            <div class="flex flex-col">
              <span class="text-base-content text-[15px]">{$t("settings.cli.title")}</span>
              <span class="text-base-content-muted text-xs">{$t("settings.cli.description")}</span>
            </div>
            <div class="flex items-center gap-2">
              {#if cliStatus?.installed}
                <button
                  class="text-base-content-muted hover:bg-base-300 hover:text-base-content rounded-lg px-3 py-1.5 text-[13px] disabled:opacity-50"
                  onclick={handleRemoveCli}
                  disabled={cliBusy}
                  type="button"
                >
                  {$t("settings.cli.remove")}
                </button>
              {/if}
              <button
                class="bg-primary text-primary-content hover:bg-primary-hover flex items-center rounded-lg px-3 py-1.5 text-[13px] disabled:opacity-50"
                onclick={handleInstallCli}
                disabled={cliBusy || !cliStatus?.bundled_path || cliStatus?.current}
                type="button"
              >
                {#if cliBusy}
                  <Loader2 size={13} class="mr-1.5 animate-spin" />
                {/if}
                {cliBusy
                  ? $t("settings.cli.installing")
                  : cliStatus?.installed
                    ? $t("settings.cli.reinstall")
                    : $t("settings.cli.install")}
              </button>
            </div>
          </div>
          {#if cliSummary}
            <span class="text-base-content-muted mt-1.5 block text-xs break-all">{cliSummary}</span>
          {/if}
          {#if cliMessage}
            <span
              class={`mt-1.5 block text-xs ${cliMessageIsError ? "text-red-500" : "text-base-content-muted"}`}
            >
              {cliMessage}
            </span>
          {/if}
        </div>

        <!-- Updates -->
        <div class="bg-base-200 rounded-2xl px-4 py-2.5">
          <div class="flex items-center justify-between">
            <div class="flex flex-col">
              <span class="text-base-content text-[15px]">{$t("settings.about")}</span>
              {#if currentVersion}
                <span class="text-base-content-muted text-xs">
                  {$t("settings.currentVersion", { version: currentVersion })}
                </span>
              {/if}
            </div>
            {#if hasUpdate}
              <div class="flex items-center gap-2">
                <span class="text-success text-xs font-medium">
                  {$t("settings.newVersionAvailable", { version: latestVersion })}
                </span>
                <button
                  class="bg-success text-success-content hover:bg-primary-hover flex items-center rounded-lg px-3 py-1.5 text-[13px] disabled:opacity-50"
                  onclick={handleInstallUpdate}
                  disabled={isInstalling}
                  type="button"
                >
                  {#if isInstalling}
                    <Loader2 size={13} class="mr-1.5 animate-spin" />
                  {:else}
                    <Download size={13} class="mr-1.5" />
                  {/if}
                  {isInstalling ? $t("settings.installingUpdate") : $t("settings.installUpdate")}
                </button>
              </div>
            {:else}
              <button
                class="bg-primary text-primary-content hover:bg-primary-hover rounded-lg px-3 py-1.5 text-[13px] disabled:opacity-50"
                onclick={handleCheckUpdate}
                disabled={checkingUpdate}
                type="button"
              >
                {checkingUpdate ? $t("settings.checkingUpdate") : $t("settings.checkUpdateBtn")}
              </button>
            {/if}
          </div>
          {#if updateMessage}
            <span class="text-base-content-muted mt-1.5 block text-xs">{updateMessage}</span>
          {/if}
        </div>
      </section>
    </div>
  </main>
</section>

<TranslateSettingsModal
  bind:open={translateSettingsOpen}
  apiKey={$settings.openrouter_api_key ?? ""}
  targetLanguage={$settings.translate_target_language || ""}
  model={$settings.translate_model || ""}
  displayMode={$settings.translate_display_mode}
  textStyle={$settings.translate_text_style}
  saving={savingTranslateSettings}
  onSave={handleSaveTranslateSettings}
/>
