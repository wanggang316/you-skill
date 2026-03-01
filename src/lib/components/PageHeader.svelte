<script>
  import {
    ChevronLeft,
    Plus,
    Settings,
    Folders,
    List,
    ArrowUpCircle,
    ExternalLink,
    RefreshCw,
    Loader2,
    Languages,
  } from "@lucide/svelte";
  import SegmentedTabs from "./ui/SegmentedTabs.svelte";
  import InstallScopeSelect from "./InstallScopeSelect.svelte";
  import { t } from "../i18n";

  const noop = () => {};
  /** @param {string} _tab */
  const noopTabChange = (_tab) => {};
  const iconButtonClass =
    "inline-flex items-center justify-center rounded-xl border border-base-300 bg-base-100 p-2 text-base-content transition hover:bg-base-300";

  let {
    currentView,
    activeTab,
    skillName,
    currentFileName = "",
    currentFilePath = "",
    hasUpdate,
    agentAppsLoading = false,
    onChangeView = undefined,
    onChangeTab = noopTabChange,
    onAddSkill = noop,
    onOpenUpdate = noop,
    onOpenProjectManage = noop,
    onOpenSettings = noop,
    updateLoading = false,
    onBack = noop,
    onDetailAction = undefined,
    onOpenCatalog = undefined,
    onRefreshAgentApps = noop,
    onTranslate = undefined,
    showTranslate = false,
    translating = false,
    translateLabel = undefined,
    localScopeKey = $bindable("global"),
    projects = [],
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
          <button
            class="bg-primary text-primary-content hover:bg-primary/80 inline-flex items-center justify-center rounded-xl p-2 transition"
            onclick={onAddSkill}
            title={$t("header.add")}
            aria-label={$t("header.add")}
            type="button"
          >
            <Plus size={16} />
          </button>
        </div>
        <div class="flex items-center gap-2">
          <InstallScopeSelect bind:value={localScopeKey} {projects} />
          <button
            class={iconButtonClass}
            onclick={onOpenProjectManage}
            title={$t("projectManage.title")}
            aria-label={$t("projectManage.title")}
            type="button"
          >
            <Folders size={16} />
          </button>
          <button
            class={iconButtonClass}
            onclick={onOpenSettings ?? onOpenUpdate}
            title={$t("header.settings")}
            aria-label={$t("header.settings")}
            type="button"
          >
            <Settings size={16} />
          </button>
          {#if hasUpdate}
            <button
              class="bg-error/10 text-error/60 hover:text-error/80 hover:bg-error/25 border-error/60 disabled:bg-error/5 flex items-center rounded-xl border p-2 text-xs transition"
              onclick={onOpenUpdate}
              disabled={updateLoading}
              title={$t("header.updateAvailable")}
              type="button"
            >
              {#if updateLoading}
                <Loader2 size={14} class="mr-1 animate-spin" />
                Updating...
              {:else}
                <ArrowUpCircle size={14} class="mr-1" />
                Update
              {/if}
            </button>
          {/if}
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
            <div class="min-w-0">
              <h1 class="text-base-content flex items-center gap-2 text-lg font-medium">
                <span class="truncate">{skillName}</span>
                {#if currentFilePath}
                  <span class="text-base-content-subtle text-sm">/</span>
                  <span class="text-base-content-subtle truncate text-sm">{currentFilePath}</span>
                {/if}
              </h1>
            </div>
          </div>
          <div class="flex items-center gap-2">
            {#if showTranslate && onTranslate}
              <button
                class="border-base-300 text-base-content hover:bg-base-200 flex h-[34px] items-center gap-2 rounded-xl border px-3 text-sm transition disabled:opacity-50"
                onclick={onTranslate}
                disabled={translating}
                type="button"
              >
                {#if translating}
                  <Loader2 size={16} class="animate-spin" />
                  {$t("detail.translating")}
                {:else}
                  <Languages size={16} />
                  {translateLabel || $t("detail.translate")}
                {/if}
              </button>
            {/if}
            {#if onOpenCatalog}
              <button
                class={iconButtonClass}
                onclick={onOpenCatalog}
                title={$t("detail.catalog")}
                aria-label={$t("detail.catalog")}
                type="button"
              >
                <List size={16} />
              </button>
            {/if}
            {#if onDetailAction}
              <button
                class={iconButtonClass}
                onclick={onDetailAction}
                title={$t("detail.openInBrowser")}
                aria-label={$t("detail.openInBrowser")}
                type="button"
              >
                <ExternalLink size={16} />
              </button>
            {/if}
          </div>
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
