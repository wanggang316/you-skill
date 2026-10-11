<script lang="ts">
  import { onMount } from "svelte";
  import { getVersion } from "@tauri-apps/api/app";
  import { open as openExternal } from "@tauri-apps/plugin-shell";
  import appIcon from "$lib/assets/app-icon.png";
  import MarkdownPreview from "$lib/components/MarkdownPreview.svelte";
  import Modal from "$lib/components/ui/Modal.svelte";
  import PrimaryActionButton from "$lib/components/ui/PrimaryActionButton.svelte";
  import { t } from "$lib/i18n";
  import { installAvailableUpdate, updateDialogOpen, updaterState } from "$lib/stores/updater";
  import { renderMarkdown } from "$lib/utils/markdown";

  /** Laid out like the Sparkle update windows: app icon on the left, text and actions on the right. */
  let open = $state(false);
  let modal = $state<ReturnType<typeof Modal>>();
  let currentVersion = $state("");

  type View = "checking" | "installing" | "available" | "error" | "upToDate";

  const view = $derived.by((): View => {
    const state = $updaterState;
    if (state.installing) return "installing";
    if (state.checking) return "checking";
    if (state.hasUpdate) return "available";
    if (state.error) return "error";
    return "upToDate";
  });
  const releaseNotes = $derived(renderMarkdown($updaterState.update?.body));
  const progress = $derived(
    $updaterState.contentLength > 0
      ? Math.min(100, ($updaterState.downloaded / $updaterState.contentLength) * 100)
      : null
  );

  const secondaryButtonClass =
    "border-base-300 text-base-content hover:bg-base-200 h-8 rounded-lg border px-4 text-[13px] transition";
  const primaryButtonClass = "h-8 rounded-lg px-4 py-0 text-[13px]";

  $effect(() => {
    open = $updateDialogOpen;
  });

  onMount(async () => {
    try {
      currentVersion = await getVersion();
    } catch (error) {
      console.error("Failed to load app version:", error);
    }
  });

  function close() {
    if (modal) modal.handleClose();
    else updateDialogOpen.set(false);
  }

  function handleInstall() {
    // The store keeps the error; the dialog shows it in the "available" view.
    installAvailableUpdate().catch(console.error);
  }

  function formatMegabytes(bytes: number): string {
    return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  }
</script>

<Modal
  bind:this={modal}
  bind:open
  showCloseButton={false}
  onClose={() => updateDialogOpen.set(false)}
  containerClass={view === "available" ? "max-w-[600px]" : "max-w-[420px]"}
>
  <div class="flex gap-5 px-6 pt-6 pb-5" data-update-view={view}>
    <img class="size-16 flex-none select-none" src={appIcon} alt="" draggable="false" />

    <div class="flex min-w-0 flex-1 flex-col">
      {#if view === "checking" || view === "installing"}
        <p class="text-base-content mt-1 text-[13px] font-semibold">
          {view === "checking" ? $t("updater.checking") : $t("updater.downloading")}
        </p>
        <div class="bg-base-300 relative mt-3 h-1.5 w-full overflow-hidden rounded-full">
          {#if progress === null}
            <div
              class="update-indeterminate bg-primary absolute inset-y-0 w-1/3 rounded-full"
            ></div>
          {:else}
            <div
              class="bg-primary h-full rounded-full transition-[width] duration-200"
              style={`width: ${progress}%`}
            ></div>
          {/if}
        </div>
        {#if view === "installing" && $updaterState.contentLength > 0}
          <p class="text-base-content-muted mt-2 font-mono text-[11px]">
            {formatMegabytes($updaterState.downloaded)} / {formatMegabytes(
              $updaterState.contentLength
            )}
          </p>
        {/if}
        {#if view === "checking"}
          <div class="mt-5 flex justify-end">
            <button class={secondaryButtonClass} type="button" onclick={close}>
              {$t("common.cancel")}
            </button>
          </div>
        {/if}
      {:else if view === "available"}
        <h3 class="text-base-content text-[14px] font-semibold">
          {$t("updater.availableTitle")}
        </h3>
        <p class="text-base-content mt-1.5 text-[12px] leading-relaxed">
          {$t("updater.availableBody", {
            latest: $updaterState.latestVersion,
            current: currentVersion,
          })}
        </p>
        <p class="text-base-content mt-4 text-[12px] font-semibold">
          {$t("updater.releaseNotes")}
        </p>
        <div
          class="border-base-300 bg-base-100 mt-1.5 h-56 overflow-y-auto rounded-md border px-4 py-3"
        >
          {#if releaseNotes}
            <MarkdownPreview htmlContent={releaseNotes} onOpenExternalLink={openExternal} />
          {:else}
            <p class="text-base-content-muted text-[12px]">{$t("updater.noReleaseNotes")}</p>
          {/if}
        </div>
        {#if $updaterState.error}
          <p class="text-error mt-2 text-[12px] break-words">
            {$t("updater.installFailed")}
            {$updaterState.error}
          </p>
        {/if}
        <div class="mt-5 flex justify-end gap-2">
          <button class={secondaryButtonClass} type="button" onclick={close}>
            {$t("updater.remindLater")}
          </button>
          <PrimaryActionButton className={primaryButtonClass} onclick={handleInstall}>
            {$t("updater.install")}
          </PrimaryActionButton>
        </div>
      {:else}
        <h3 class="text-base-content mt-1 text-[14px] font-semibold">
          {view === "error" ? $t("updater.errorTitle") : $t("updater.upToDateTitle")}
        </h3>
        <p class="text-base-content mt-1.5 text-[12px] leading-relaxed break-words">
          {#if view === "error"}
            {$t("updater.checkFailed")}
            <span class="text-base-content-muted">{$updaterState.error}</span>
          {:else}
            {$t("updater.upToDateBody", { version: currentVersion })}
          {/if}
        </p>
        <div class="mt-5 flex justify-end">
          <PrimaryActionButton className={`${primaryButtonClass} min-w-20`} onclick={close}>
            {$t("updater.ok")}
          </PrimaryActionButton>
        </div>
      {/if}
    </div>
  </div>
</Modal>

<style>
  .update-indeterminate {
    animation: update-indeterminate 1.2s ease-in-out infinite;
  }

  @keyframes update-indeterminate {
    from {
      left: -33%;
    }
    to {
      left: 100%;
    }
  }
</style>
