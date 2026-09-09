<script lang="ts">
  import {
    ArrowUpCircle,
    FolderTree,
    LibraryBig,
    Loader2,
    Plus,
    ScrollText,
    Settings,
    Store,
  } from "@lucide/svelte";
  import { t } from "$lib/i18n";
  import {
    buildInstructionsHref,
    buildLibraryHref,
    buildMarketHref,
    buildScopeHref,
    type SidebarActiveKey,
  } from "$lib/navigation/app-shell";

  let {
    activeKey,
    hasUpdate,
    updateLoading,
    onImportSkill,
    onOpenUpdate,
  }: {
    activeKey: SidebarActiveKey;
    hasUpdate: boolean;
    updateLoading: boolean;
    onImportSkill: () => void;
    onOpenUpdate: () => void;
  } = $props();

  const itemClass =
    "text-base-content/80 hover:bg-base-300 hover:text-base-content focus-visible:outline-primary/60 flex w-full min-w-0 items-center gap-2.5 rounded-lg px-2.5 py-2 text-left text-sm leading-5 font-normal transition-[background-color,color,transform] duration-150 focus-visible:outline-2 focus-visible:outline-offset-1 active:translate-y-px";
</script>

<aside
  class="bg-base-200 border-base-300 flex min-w-0 flex-col border-r"
  aria-label={$t("sidebar.navigation")}
  data-window-drag-region
>
  <nav class="flex min-h-0 flex-1 flex-col px-2.5 pt-10 pb-4 max-[832px]:px-[0.45rem]">
    <div class="grid gap-0.5" aria-label={$t("sidebar.skills")}>
      <button class={itemClass} type="button" onclick={onImportSkill}>
        <Plus size={17} strokeWidth={1.8} />
        <span class="min-w-0 truncate">{$t("import.title")}</span>
      </button>
      <a
        class={itemClass}
        class:bg-base-300={activeKey === "library"}
        class:text-base-content={activeKey === "library"}
        class:font-medium={activeKey === "library"}
        href={buildLibraryHref()}
        aria-current={activeKey === "library" ? "page" : undefined}
      >
        <LibraryBig size={17} strokeWidth={1.8} />
        <span class="min-w-0 truncate">{$t("sidebar.library")}</span>
      </a>
      <a
        class={itemClass}
        class:bg-base-300={activeKey === "instructions"}
        class:text-base-content={activeKey === "instructions"}
        class:font-medium={activeKey === "instructions"}
        href={buildInstructionsHref()}
        aria-current={activeKey === "instructions" ? "page" : undefined}
      >
        <ScrollText size={17} strokeWidth={1.8} />
        <span class="min-w-0 truncate">{$t("sidebar.instructions")}</span>
      </a>
      <a
        class={itemClass}
        class:bg-base-300={activeKey === "projects"}
        class:text-base-content={activeKey === "projects"}
        class:font-medium={activeKey === "projects"}
        href={buildScopeHref()}
        aria-current={activeKey === "projects" ? "page" : undefined}
      >
        <FolderTree size={17} strokeWidth={1.8} />
        <span class="min-w-0 truncate">{$t("sidebar.projects")}</span>
      </a>
      <a
        class={itemClass}
        class:bg-base-300={activeKey === "market"}
        class:text-base-content={activeKey === "market"}
        class:font-medium={activeKey === "market"}
        href={buildMarketHref()}
        aria-current={activeKey === "market" ? "page" : undefined}
      >
        <Store size={17} strokeWidth={1.8} />
        <span class="min-w-0 truncate">{$t("sidebar.market")}</span>
      </a>
    </div>
  </nav>

  <div class="border-base-300 grid gap-0.5 border-t p-2.5 max-[832px]:px-[0.45rem]">
    {#if hasUpdate}
      <button
        class="text-error hover:bg-base-300 focus-visible:outline-primary/60 flex w-full min-w-0 items-center gap-2.5 rounded-lg px-2.5 py-2 text-left text-sm leading-5 font-normal transition-[background-color,color,transform] duration-150 focus-visible:outline-2 focus-visible:outline-offset-1 active:translate-y-px disabled:cursor-default disabled:opacity-55"
        type="button"
        onclick={onOpenUpdate}
        disabled={updateLoading}
      >
        {#if updateLoading}
          <Loader2 size={17} class="animate-spin" />
        {:else}
          <ArrowUpCircle size={17} strokeWidth={1.8} />
        {/if}
        <span class="min-w-0 truncate">{$t("header.updateAvailable")}</span>
      </button>
    {/if}
    <a
      class={itemClass}
      class:bg-base-300={activeKey === "settings"}
      class:text-base-content={activeKey === "settings"}
      class:font-medium={activeKey === "settings"}
      href="/settings"
      aria-current={activeKey === "settings" ? "page" : undefined}
    >
      <Settings size={17} strokeWidth={1.8} />
      <span class="min-w-0 truncate">{$t("header.settings")}</span>
    </a>
  </div>
</aside>
