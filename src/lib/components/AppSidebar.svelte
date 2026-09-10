<script lang="ts">
  import {
    ArrowUpCircle,
    ChevronLeft,
    ChevronRight,
    FolderTree,
    LibraryBig,
    Loader2,
    Plus,
    ScrollText,
    Settings,
    Store,
  } from "@lucide/svelte";
  import { t } from "$lib/i18n";
  import { canGoBack, canGoForward, goBack, goForward } from "$lib/navigation/history";
  import {
    buildInstructionsHref,
    buildLibraryHref,
    buildMarketHref,
    buildScopeHref,
    type SidebarActiveKey,
  } from "$lib/navigation/app-shell";

  let {
    activeKey,
    collapsed = false,
    hasUpdate,
    updateLoading,
    onImportSkill,
    onOpenUpdate,
  }: {
    activeKey: SidebarActiveKey;
    /** Icons only; labels become tooltips. */
    collapsed?: boolean;
    hasUpdate: boolean;
    updateLoading: boolean;
    onImportSkill: () => void;
    onOpenUpdate: () => void;
  } = $props();

  const itemClass = $derived(
    `text-base-content/80 hover:bg-base-300 hover:text-base-content focus-visible:outline-primary/60 flex w-full min-w-0 items-center gap-2.5 rounded-lg py-2 text-left text-sm leading-5 font-normal transition-[background-color,color,transform] duration-150 focus-visible:outline-2 focus-visible:outline-offset-1 active:translate-y-px ${
      collapsed ? "justify-center px-0" : "px-2.5"
    }`
  );
  const historyButtonClass =
    "text-base-content-muted hover:bg-base-300 hover:text-base-content flex size-7 items-center justify-center rounded-lg transition disabled:opacity-35 disabled:hover:bg-transparent";
  const padding = $derived(collapsed ? "px-2" : "px-2.5");
</script>

<aside
  class="bg-base-200 border-base-300 flex min-w-0 flex-col border-r"
  aria-label={$t("sidebar.navigation")}
  data-window-drag-region
>
  {#snippet historyButtons()}
    <button
      class={historyButtonClass}
      type="button"
      onclick={goBack}
      disabled={!$canGoBack}
      title={$t("sidebar.back")}
      aria-label={$t("sidebar.back")}
    >
      <ChevronLeft size={17} strokeWidth={1.8} />
    </button>
    <button
      class={historyButtonClass}
      type="button"
      onclick={goForward}
      disabled={!$canGoForward}
      title={$t("sidebar.forward")}
      aria-label={$t("sidebar.forward")}
    >
      <ChevronRight size={17} strokeWidth={1.8} />
    </button>
  {/snippet}

  <!-- The window controls sit at the left of this row; the buttons keep to the right. -->
  <header class={`flex h-12 flex-none items-center justify-end gap-0.5 ${padding}`}>
    {#if !collapsed}
      {@render historyButtons()}
    {/if}
  </header>
  {#if collapsed}
    <div class="flex flex-none flex-col items-center gap-0.5 pb-1">
      {@render historyButtons()}
    </div>
  {/if}
  <nav class={`flex min-h-0 flex-1 flex-col pt-1 pb-4 ${padding}`}>
    <div class="grid gap-0.5" aria-label={$t("sidebar.skills")}>
      <button class={itemClass} type="button" onclick={onImportSkill} title={$t("import.title")}>
        <Plus size={17} strokeWidth={1.8} />
        {#if !collapsed}<span class="min-w-0 truncate">{$t("import.title")}</span>{/if}
      </button>
      <a
        class={itemClass}
        class:bg-base-300={activeKey === "library"}
        class:text-base-content={activeKey === "library"}
        class:font-medium={activeKey === "library"}
        href={buildLibraryHref()}
        aria-current={activeKey === "library" ? "page" : undefined}
        title={$t("sidebar.library")}
      >
        <LibraryBig size={17} strokeWidth={1.8} />
        {#if !collapsed}<span class="min-w-0 truncate">{$t("sidebar.library")}</span>{/if}
      </a>
      <a
        class={itemClass}
        class:bg-base-300={activeKey === "instructions"}
        class:text-base-content={activeKey === "instructions"}
        class:font-medium={activeKey === "instructions"}
        href={buildInstructionsHref()}
        aria-current={activeKey === "instructions" ? "page" : undefined}
        title={$t("sidebar.instructions")}
      >
        <ScrollText size={17} strokeWidth={1.8} />
        {#if !collapsed}<span class="min-w-0 truncate">{$t("sidebar.instructions")}</span>{/if}
      </a>
      <a
        class={itemClass}
        class:bg-base-300={activeKey === "projects"}
        class:text-base-content={activeKey === "projects"}
        class:font-medium={activeKey === "projects"}
        href={buildScopeHref()}
        aria-current={activeKey === "projects" ? "page" : undefined}
        title={$t("sidebar.projects")}
      >
        <FolderTree size={17} strokeWidth={1.8} />
        {#if !collapsed}<span class="min-w-0 truncate">{$t("sidebar.projects")}</span>{/if}
      </a>
      <a
        class={itemClass}
        class:bg-base-300={activeKey === "market"}
        class:text-base-content={activeKey === "market"}
        class:font-medium={activeKey === "market"}
        href={buildMarketHref()}
        aria-current={activeKey === "market" ? "page" : undefined}
        title={$t("sidebar.market")}
      >
        <Store size={17} strokeWidth={1.8} />
        {#if !collapsed}<span class="min-w-0 truncate">{$t("sidebar.market")}</span>{/if}
      </a>
    </div>
  </nav>

  <div class={`border-base-300 grid gap-0.5 border-t py-2.5 ${padding}`}>
    {#if hasUpdate}
      <button
        class={`text-error hover:bg-base-300 focus-visible:outline-primary/60 flex w-full min-w-0 items-center gap-2.5 rounded-lg py-2 text-left text-sm leading-5 font-normal transition-[background-color,color,transform] duration-150 focus-visible:outline-2 focus-visible:outline-offset-1 active:translate-y-px disabled:cursor-default disabled:opacity-55 ${
          collapsed ? "justify-center px-0" : "px-2.5"
        }`}
        type="button"
        onclick={onOpenUpdate}
        disabled={updateLoading}
        title={$t("header.updateAvailable")}
      >
        {#if updateLoading}
          <Loader2 size={17} class="animate-spin" />
        {:else}
          <ArrowUpCircle size={17} strokeWidth={1.8} />
        {/if}
        {#if !collapsed}<span class="min-w-0 truncate">{$t("header.updateAvailable")}</span>{/if}
      </button>
    {/if}
    <a
      class={itemClass}
      class:bg-base-300={activeKey === "settings"}
      class:text-base-content={activeKey === "settings"}
      class:font-medium={activeKey === "settings"}
      href="/settings"
      aria-current={activeKey === "settings" ? "page" : undefined}
      title={$t("header.settings")}
    >
      <Settings size={17} strokeWidth={1.8} />
      {#if !collapsed}<span class="min-w-0 truncate">{$t("header.settings")}</span>{/if}
    </a>
  </div>
</aside>
