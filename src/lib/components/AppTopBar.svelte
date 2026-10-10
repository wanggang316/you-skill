<script lang="ts">
  import {
    ArrowUpCircle,
    FolderTree,
    LibraryBig,
    Loader2,
    ScrollText,
    Settings,
    Store,
    Zap,
  } from "@lucide/svelte";
  import type { Component } from "svelte";
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
    onOpenLibrary,
    onOpenUpdate,
  }: {
    activeKey: SidebarActiveKey;
    hasUpdate: boolean;
    updateLoading: boolean;
    onOpenLibrary: () => void;
    onOpenUpdate: () => void;
  } = $props();

  type NavItem = {
    key: SidebarActiveKey;
    href: string;
    label: string;
    icon: Component<{ size?: number; strokeWidth?: number }>;
    onclick?: () => void;
  };

  /** Local content first, then the marketplace as its own group. */
  const groups: NavItem[][] = $derived([
    [
      {
        key: "library",
        href: buildLibraryHref(),
        label: $t("sidebar.library"),
        icon: LibraryBig,
        onclick: onOpenLibrary,
      },
      {
        key: "instructions",
        href: buildInstructionsHref(),
        label: $t("sidebar.instructions"),
        icon: ScrollText,
      },
      { key: "projects", href: buildScopeHref(), label: $t("sidebar.projects"), icon: FolderTree },
    ],
    [{ key: "market", href: buildMarketHref(), label: $t("sidebar.market"), icon: Store }],
  ]);

  const itemClass =
    "text-base-content/70 hover:text-base-content focus-visible:outline-primary/60 flex h-7 items-center gap-1.5 rounded-md px-3 text-[13px] leading-5 transition-[background-color,color,box-shadow] duration-150 focus-visible:outline-2 focus-visible:outline-offset-1";
  const activeClass = "bg-base-100 text-base-content font-medium shadow-sm";
  const actionClass =
    "focus-visible:outline-primary/60 flex h-8 items-center gap-1.5 rounded-lg transition-[background-color,color] duration-150 focus-visible:outline-2 focus-visible:outline-offset-1 active:translate-y-px";
</script>

<!-- The window controls sit at the left of this row (centred by `trafficLightPosition`). -->
<header
  class="bg-base-200 border-base-300 grid h-12 flex-none grid-cols-[1fr_auto_1fr] items-center gap-4 border-b pr-3 pl-[5.25rem]"
  data-window-drag-region
>
  <div class="flex min-w-0 items-center gap-2">
    <span
      class="bg-neutral text-neutral-content flex size-6 flex-none items-center justify-center rounded-md"
      aria-hidden="true"
    >
      <Zap size={14} strokeWidth={2.2} />
    </span>
    <span class="text-base-content truncate text-[15px] font-semibold tracking-[-0.01em] italic">
      YouSkill
    </span>
  </div>

  <nav class="flex items-center gap-2" aria-label={$t("sidebar.navigation")}>
    {#each groups as group, index}
      {#if index > 0}
        <span class="bg-base-300 h-4 w-px" aria-hidden="true"></span>
      {/if}
      <div class="bg-base-300/70 flex items-center gap-0.5 rounded-lg p-0.5">
        {#each group as item (item.key)}
          {@const active = activeKey === item.key}
          {@const Icon = item.icon}
          <a
            class={`${itemClass} ${active ? activeClass : ""}`}
            href={item.href}
            onclick={item.onclick}
            aria-current={active ? "page" : undefined}
          >
            <Icon size={14} strokeWidth={1.8} />
            <span class="whitespace-nowrap">{item.label}</span>
          </a>
        {/each}
      </div>
    {/each}
  </nav>

  <div class="flex min-w-0 items-center justify-end gap-1">
    {#if hasUpdate}
      <button
        class={`${actionClass} text-error hover:bg-base-300 px-2.5 text-[13px] disabled:cursor-default disabled:opacity-55`}
        type="button"
        onclick={onOpenUpdate}
        disabled={updateLoading}
        title={$t("header.updateAvailable")}
      >
        {#if updateLoading}
          <Loader2 size={15} class="animate-spin" />
        {:else}
          <ArrowUpCircle size={15} strokeWidth={1.8} />
        {/if}
        <span class="truncate">{$t("header.update")}</span>
      </button>
    {/if}
    <a
      class={`${actionClass} w-8 justify-center ${
        activeKey === "settings"
          ? "bg-base-300 text-base-content"
          : "text-base-content/70 hover:bg-base-300 hover:text-base-content"
      }`}
      href="/settings"
      aria-current={activeKey === "settings" ? "page" : undefined}
      title={$t("header.settings")}
      aria-label={$t("header.settings")}
    >
      <Settings size={17} strokeWidth={1.8} />
    </a>
  </div>
</header>
