<script>
  import {
    ChevronLeft,
    Plus,
    Settings,
    Download,
    ArrowUpCircle,
  } from "@lucide/svelte";
  import IconButton from "./IconButton.svelte";
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
  } = $props();
</script>

<header
  class="sticky top-0 z-50 border-b border-[var(--base-300)] bg-[var(--base-100)]"
>
  <div class="mx-auto flex max-w-6xl items-center justify-between px-6 py-4">
    {#if currentView === "list"}
      <div class="flex items-center gap-2 text-[var(--base-content)]">
        <svg
          width="28"
          viewBox="0 0 433 455"
          fill="none"
          xmlns="http://www.w3.org/2000/svg"
        >
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
          <div
            class="flex items-center gap-2 rounded-full bg-[var(--base-200)] p-1"
          >
            <button
              class={`rounded-full px-4 py-2 transition hover:text-[var(--base-content)] ${activeTab === "local" ? "bg-[var(--base-100)] text-[var(--base-content)] shadow-sm" : "text-[var(--base-content-muted)]"}`}
              onclick={() => onChangeTab("local")}
              type="button"
            >
              {$t("header.localTab")}
            </button>
            <button
              class={`rounded-full px-4 py-2 transition hover:text-[var(--base-content)] ${activeTab === "remote" ? "bg-[var(--base-100)] text-[var(--base-content)] shadow-sm" : "text-[var(--base-content-muted)]"}`}
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
              class="flex items-center gap-1.5 rounded-xl bg-[var(--warning)] px-3 py-2 text-sm font-medium text-[var(--warning-content)] transition hover:opacity-90"
              onclick={onOpenPendingImport}
              title={$t("header.pendingImport")}
              type="button"
            >
              <Download size={16} />
              {$t("header.pendingImport")}
              <span
                class="ml-1 rounded-full bg-[var(--warning-content)] px-1.5 py-0.5 text-xs text-[var(--warning)]"
                >{unmanagedCount}</span
              >
            </button>
          {/if}
          {#if hasUpdate}
            <button
              class="flex items-center gap-1.5 rounded-xl bg-[var(--success)] px-3 py-2 text-sm font-medium text-[var(--success-content)] transition hover:opacity-90"
              onclick={onOpenUpdate}
              title={$t("header.update")}
              type="button"
            >
              <ArrowUpCircle size={16} />
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
        <div class="flex items-center gap-4">
          <button
            class="flex items-center gap-2 rounded-xl border border-[var(--base-300)] px-3 py-2 text-sm text-[var(--base-content)] transition hover:bg-[var(--base-200)]"
            onclick={onBack}
            title={$t("header.back")}
            type="button"
          >
            <ChevronLeft size={16} />
            {$t("header.back")}
          </button>
          <h1 class="text-lg font-semibold text-[var(--base-content)]">
            {skillName}
          </h1>
        </div>
      {:else}
        <div class="flex items-center gap-4">
          <button
            class="flex items-center gap-2 rounded-xl border border-[var(--base-300)] px-3 py-2 text-sm text-[var(--base-content)] transition hover:bg-[var(--base-200)]"
            onclick={() => onChangeView("list")}
            title={$t("header.back")}
            type="button"
          >
            <ChevronLeft size={16} />
            {$t("header.back")}
          </button>
          <h1 class="text-lg font-semibold text-[var(--base-content)]">
            {$t("header.settings")}
          </h1>
        </div>
      {/if}
    </div>
  </div>
</header>
