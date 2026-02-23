<script>
  import { goto } from "$app/navigation";
  import {
    ChevronLeft,
    Plus,
    Settings,
    ArrowUpCircle,
    ExternalLink,
    RefreshCw,
    Loader2,
  } from "@lucide/svelte";
  import IconButton from "./ui/IconButton.svelte";
  import SegmentedTabs from "./ui/SegmentedTabs.svelte";
  import { t } from "../i18n";

  const {
    currentView,
    activeTab,
    skillName,
    hasUpdate,
    agentAppsLoading = false,
    onChangeView = undefined,
    onChangeTab,
    onAddSkill,
    onOpenUpdate,
    onOpenSettings,
    updateLoading = false,
    onBack,
    onDetailAction,
    onRefreshAgentApps,
  } = $props();
</script>

<header class="border-base-300 bg-base-100 sticky top-0 z-50 border-b">
  <div class="mx-auto flex max-w-6xl items-center justify-between px-6 py-4">
    {#if currentView === "list"}
      <div class="text-base-content flex items-center gap-2">
        <svg width="28" viewBox="0 0 433 455" fill="none" xmlns="http://www.w3.org/2000/svg">
          <path
            d="M0 263.506L265.588 0L205.547 180.705H433L153.889 455L217.987 263.506H0Z"
            fill="currentColor"
          />
        </svg>
        <p class="text-lg italic">YouSkill</p>
      </div>
    {/if}
    <div
      class="flex flex-1 items-center justify-between {currentView === 'list'
        ? 'pl-6'
        : ''} text-sm"
    >
      {#if currentView === "list"}
        <div class="flex items-center gap-3">
          <SegmentedTabs
            items={[
              { value: "local", label: $t("header.localTab") },
              { value: "remote", label: $t("header.remoteTab") },
            ]}
            value={activeTab}
            onChange={onChangeTab}
          />
          <IconButton
            variant="primary"
            onclick={onAddSkill}
            title={$t("header.add")}
            ariaLabel={$t("header.add")}
          >
            <Plus size={16} />
          </IconButton>
        </div>
        <div class="flex items-center gap-2">
          {#if hasUpdate}
            <button
              class="bg-error/10 text-warning hover:bg-error/25 border-error/20 disabled:bg-error/5 flex items-center rounded-lg border px-2 py-1 text-xs transition"
              onclick={onOpenUpdate}
              disabled={updateLoading}
              title={$t("header.updateAvailable")}
              type="button"
            >
              {#if updateLoading}
                <Loader2 size={14} class="mr-1 animate-spin" />
                {$t("settings.installingUpdate")}
              {:else}
                <ArrowUpCircle size={14} class="mr-1" />
                {$t("header.updateAvailable")}
              {/if}
            </button>
          {/if}
          <IconButton
            variant="outline"
            onclick={onOpenSettings ?? onOpenUpdate}
            title={$t("header.settings")}
            ariaLabel={$t("header.settings")}
          >
            <Settings size={16} />
          </IconButton>
        </div>
      {:else if currentView === "agentApps"}
        <div class="flex w-full items-center justify-between">
          <div class="flex items-center gap-4">
            <button
              class="border-base-300 text-base-content hover:bg-base-200 flex items-center rounded-xl border p-2 text-sm transition"
              onclick={onBack}
              title={$t("header.back")}
              type="button"
            >
              <ChevronLeft size={16} />
            </button>
            <h1 class="text-base-content text-lg font-medium">
              {$t("agentApps.title")}
            </h1>
          </div>
          <button
            class="border-base-300 text-base-content hover:bg-base-200 flex items-center gap-2 rounded-xl border px-3 py-2 text-sm transition disabled:opacity-50"
            onclick={onRefreshAgentApps}
            disabled={agentAppsLoading}
            type="button"
          >
            {#if agentAppsLoading}
              <Loader2 size={16} class="animate-spin" />
            {:else}
              <RefreshCw size={16} />
            {/if}
            {$t("local.refresh")}
          </button>
        </div>
      {:else if currentView === "detail"}
        <div class="flex w-full items-center justify-between">
          <div class="flex items-center gap-4">
            <button
              class="border-base-300 text-base-content hover:bg-base-200 flex items-center rounded-xl border p-2 text-sm transition"
              onclick={onBack}
              title={$t("header.back")}
              type="button"
            >
              <ChevronLeft size={16} />
            </button>
            <h1 class="text-base-content text-lg font-medium">
              {skillName}
            </h1>
          </div>
          {#if onDetailAction}
            <IconButton
              variant="outline"
              onclick={onDetailAction}
              title={$t("detail.openInBrowser")}
              ariaLabel={$t("detail.openInBrowser")}
            >
              <ExternalLink size={16} />
            </IconButton>
          {/if}
        </div>
      {:else}
        <div class="flex items-center gap-4">
          <button
            class="border-base-300 text-base-content hover:bg-base-200 flex items-center rounded-xl border p-2 text-sm transition"
            onclick={onChangeView ? () => onChangeView("list") : onBack}
            title={$t("header.back")}
            type="button"
          >
            <ChevronLeft size={16} />
          </button>
          <h1 class="text-base-content text-lg font-medium">
            {$t("header.settings")}
          </h1>
        </div>
      {/if}
    </div>
  </div>
</header>
