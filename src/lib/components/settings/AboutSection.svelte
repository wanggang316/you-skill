<script lang="ts">
  import { onMount } from "svelte";
  import { getVersion } from "@tauri-apps/api/app";
  import { open as openExternal } from "@tauri-apps/plugin-shell";
  import { ArrowUpRight, Download } from "@lucide/svelte";
  import PrimaryActionButton from "$lib/components/ui/PrimaryActionButton.svelte";
  import { BRAND_BOLT_PATH, BRAND_BOLT_VIEW_BOX, WEBSITE_URL } from "$lib/brand";
  import { t } from "$lib/i18n";
  import { openUpdateDialog, updaterState } from "$lib/stores/updater";

  let currentVersion = $state("");

  const buttonClass = "h-8 rounded-lg px-4 py-0 text-[13px]";

  onMount(async () => {
    try {
      currentVersion = await getVersion();
    } catch (error) {
      console.error("Failed to load app version:", error);
    }
  });
</script>

<div class="flex min-h-0 flex-col items-center overflow-y-auto px-8 pt-[16vh] pb-12 text-center">
  <svg
    class="text-base-content h-10 w-auto"
    viewBox={BRAND_BOLT_VIEW_BOX}
    fill="currentColor"
    aria-hidden="true"
  >
    <path d={BRAND_BOLT_PATH} />
  </svg>
  <h2 class="text-base-content mt-4 text-[1.5rem] font-semibold tracking-[-0.03em] italic">
    YouSkill
  </h2>
  {#if currentVersion}
    <p class="text-base-content-muted mt-1 font-mono text-[12px]">
      {$t("settings.currentVersion", { version: currentVersion })}
    </p>
  {/if}
  <p class="text-base-content-muted mt-3 max-w-sm text-[13px] leading-relaxed">
    {$t("settings.about.tagline")}
  </p>

  <div class="mt-7">
    <PrimaryActionButton className={buttonClass} onclick={openUpdateDialog}>
      {#if $updaterState.hasUpdate}
        <Download size={14} />
        {$t("settings.installUpdate")} v{$updaterState.latestVersion}
      {:else}
        {$t("settings.checkUpdateBtn")}
      {/if}
    </PrimaryActionButton>
  </div>

  <button
    class="text-base-content-muted hover:text-base-content mt-8 inline-flex cursor-pointer items-center gap-0.5 text-[12px] transition-colors"
    type="button"
    onclick={() => openExternal(WEBSITE_URL)}
  >
    {$t("settings.about.website")}
    <ArrowUpRight size={13} />
  </button>
</div>
