<script>
  import {
    ChevronLeft,
    Plus,
    Settings,
    Download,
    ArrowUpCircle,
    ExternalLink,
  } from "@lucide/svelte";
  import IconButton from "./ui/IconButton.svelte";
  import { t } from "../i18n";

  const {
    currentView,
    activeTab,
    skillName,
    unmanagedCount,
    hasUpdate,
    onChangeView,
    onChangeTab,
    onAddSkill,
    onOpenPendingImport,
    onOpenUpdate,
    onBack,
    onDetailAction,
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
          <div class="bg-base-200 flex items-center gap-2 rounded-full p-1">
            <button
              class={`hover:text-base-content rounded-full px-4 py-2 transition ${activeTab === "local" ? "bg-base-100 text-base-content shadow-sm" : "text-base-content-muted"}`}
              onclick={() => onChangeTab("local")}
              type="button"
            >
              {$t("header.localTab")}
            </button>
            <button
              class={`hover:text-base-content rounded-full px-4 py-2 transition ${activeTab === "remote" ? "bg-base-100 text-base-content shadow-sm" : "text-base-content-muted"}`}
              onclick={() => onChangeTab("remote")}
              type="button"
            >
              {$t("header.remoteTab")}
            </button>
          </div>
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
          {#if unmanagedCount > 0}
            <button
              class="bg-warning text-warning-content hover:bg-warning-hover flex items-center rounded-xl px-3 py-1.5 text-sm font-medium transition"
              onclick={onOpenPendingImport}
              title={$t("header.pendingImport")}
              type="button"
            >
              <Download size={16} class="mr-1.5" />
              {$t("header.pendingImport")}
              <span class="bg-warning-content text-warning ml-1 rounded-full px-1.5 py-0.5 text-xs"
                >{unmanagedCount}</span
              >
            </button>
          {/if}
          {#if hasUpdate}
            <button
              class="bg-success text-success-content hover:bg-success-hover flex items-center rounded-xl px-3 py-2 text-sm font-medium transition"
              onclick={onOpenUpdate}
              title={$t("header.update")}
              type="button"
            >
              <ArrowUpCircle size={16} class="mr-1.5" />
              {$t("header.update")}
            </button>
          {/if}
          <IconButton
            variant="outline"
            onclick={() => onChangeView("settings")}
            title={$t("header.settings")}
            ariaLabel={$t("header.settings")}
          >
            <Settings size={16} />
          </IconButton>
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
            <h1 class="text-base-content text-lg font-semibold">
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
            onclick={() => onChangeView("list")}
            title={$t("header.back")}
            type="button"
          >
            <ChevronLeft size={16} class="mr-0" />
          </button>
          <h1 class="text-base-content text-lg font-medium">
            {$t("header.settings")}
          </h1>
        </div>
      {/if}
    </div>
  </div>
</header>
