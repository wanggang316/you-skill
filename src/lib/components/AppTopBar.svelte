<script lang="ts">
  import { onMount } from "svelte";
  import {
    FolderTree,
    LibraryBig,
    Loader2,
    Plus,
    ScrollText,
    Settings,
    Store,
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
    latestVersion = "",
    onImport,
    onOpenLibrary,
    onOpenUpdate,
  }: {
    activeKey: SidebarActiveKey;
    hasUpdate: boolean;
    updateLoading: boolean;
    latestVersion?: string;
    onImport: () => void;
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

  /** Local content; the marketplace sits apart as an icon button. */
  const items: NavItem[] = $derived([
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
  ]);

  const marketActive = $derived(activeKey === "market");
  /** Recessed track shared by the nav and the icon buttons; the active item floats above it. */
  const trackClass =
    "bg-base-content/[0.055] shadow-[inset_0_0_0_0.5px_color-mix(in_oklch,var(--color-base-content)_4%,transparent)]";
  const raisedClass =
    "bg-raised shadow-[0_1px_2px_0_color-mix(in_oklch,var(--color-base-content)_10%,transparent),0_0_0_0.5px_color-mix(in_oklch,var(--color-base-content)_8%,transparent)]";

  let nav = $state<HTMLElement | null>(null);
  /** Position of the active pill; it slides between items instead of jumping. */
  let indicator = $state({ left: 0, width: 0 });
  /** Hidden on pages outside the tabs (market, settings); keeps its last spot to fade out in place. */
  let indicatorVisible = $state(false);
  /** No slide on first paint, only on later moves. */
  let animate = $state(false);

  function measure() {
    const active = nav?.querySelector<HTMLElement>(`[data-nav-key="${activeKey}"]`);
    // Only write here, never read: this runs inside an effect.
    indicatorVisible = Boolean(active);
    if (active) indicator = { left: active.offsetLeft, width: active.offsetWidth };
  }

  $effect(() => {
    // Re-measure when the route or the labels (locale) change.
    void activeKey;
    void items;
    measure();
  });

  onMount(() => {
    const observer = new ResizeObserver(measure);
    if (nav) observer.observe(nav);
    requestAnimationFrame(() => (animate = true));
    return () => observer.disconnect();
  });
</script>

<!-- The window controls sit at the left of this row (centred by `trafficLightPosition`). -->
<header
  class="grid h-15 flex-none grid-cols-[1fr_auto_1fr] items-center gap-4 pr-4 pl-[5.75rem]"
  data-window-drag-region
>
  <a
    class="group focus-visible:ring-primary/50 flex w-fit min-w-0 items-center gap-1.5 rounded-lg focus-visible:ring-2 focus-visible:outline-none"
    href={buildLibraryHref()}
    onclick={onOpenLibrary}
    aria-label="YouSkill"
  >
    <svg
      class="text-base-content h-[25px] w-auto flex-none transition-transform duration-300 ease-out group-hover:-rotate-12"
      viewBox="0 0 433 455"
      fill="currentColor"
      aria-hidden="true"
    >
      <path d="M0 263.506L265.588 0L205.547 180.705H433L153.889 455L217.987 263.506H0Z" />
    </svg>
    <span
      class="text-base-content truncate pr-1 text-[16px] font-medium tracking-[-0.03em] italic"
    >
      YouSkill
    </span>
  </a>

  <div class="flex items-center gap-2">
    <nav
      bind:this={nav}
      class={`${trackClass} relative flex h-9 items-center gap-0.5 rounded-full p-[3px]`}
      aria-label={$t("sidebar.navigation")}
    >
      <span
        class={`${raisedClass} absolute top-[3px] bottom-[3px] left-0 rounded-full ${
          animate
            ? "transition-[transform,width,opacity] duration-300 ease-[cubic-bezier(0.32,0.72,0,1)]"
            : ""
        }`}
        style={`transform: translateX(${indicator.left}px); width: ${indicator.width}px; opacity: ${
          indicatorVisible ? 1 : 0
        };`}
        aria-hidden="true"
      ></span>
      {#each items as item (item.key)}
        {@const active = activeKey === item.key}
        {@const Icon = item.icon}
        <a
          class={`focus-visible:ring-primary/50 relative z-10 flex h-full items-center gap-1.5 rounded-full px-3 text-[13px] whitespace-nowrap transition-colors duration-200 focus-visible:ring-2 focus-visible:outline-none ${
            active
              ? "text-base-content font-medium"
              : "text-base-content-muted hover:text-base-content"
          }`}
          href={item.href}
          onclick={item.onclick}
          data-nav-key={item.key}
          aria-current={active ? "page" : undefined}
        >
          <Icon size={15} strokeWidth={1.8} />
          {item.label}
        </a>
      {/each}
    </nav>

    <a
      class={`${trackClass} focus-visible:ring-primary/50 flex size-9 items-center justify-center rounded-full p-[3px] transition-colors duration-200 focus-visible:ring-2 focus-visible:outline-none ${
        marketActive ? "text-base-content" : "text-base-content-muted hover:text-base-content"
      }`}
      href={buildMarketHref()}
      aria-current={marketActive ? "page" : undefined}
      title={$t("sidebar.market")}
      aria-label={$t("sidebar.market")}
    >
      <span
        class={`flex size-full items-center justify-center rounded-full transition-[background-color,box-shadow] duration-200 ${
          marketActive ? raisedClass : ""
        }`}
      >
        <Store size={16} strokeWidth={1.8} />
      </span>
    </a>

    <button
      class={`${trackClass} focus-visible:ring-primary/50 text-base-content-muted hover:text-base-content flex size-9 items-center justify-center rounded-full transition-[color,transform] duration-200 focus-visible:ring-2 focus-visible:outline-none active:scale-95`}
      type="button"
      onclick={onImport}
      title={$t("import.title")}
      aria-label={$t("import.title")}
    >
      <Plus size={17} strokeWidth={1.8} />
    </button>
  </div>

  <div class="flex min-w-0 items-center justify-end gap-1.5">
    {#if hasUpdate}
      <button
        class="border-primary/25 bg-primary/8 text-primary hover:bg-primary/14 flex h-8 min-w-0 items-center gap-1.5 rounded-full border px-2.5 text-[12px] font-medium transition-colors duration-150 disabled:cursor-default disabled:opacity-60"
        type="button"
        onclick={onOpenUpdate}
        disabled={updateLoading}
        title={$t("header.updateAvailable")}
      >
        {#if updateLoading}
          <Loader2 size={12} class="animate-spin" />
        {:else}
          <span class="relative flex size-1.5 flex-none" aria-hidden="true">
            <span class="bg-primary absolute inset-0 animate-ping rounded-full opacity-60"></span>
            <span class="bg-primary relative size-1.5 rounded-full"></span>
          </span>
        {/if}
        <span class="whitespace-nowrap">{$t("header.update")}</span>
        {#if latestVersion}
          <span class="whitespace-nowrap opacity-70 max-[1040px]:hidden">v{latestVersion}</span>
        {/if}
      </button>
    {/if}
    <a
      class={`flex size-9 items-center justify-center rounded-full transition-[background-color,color,transform] duration-200 active:scale-95 ${
        activeKey === "settings"
          ? "bg-base-content/[0.07] text-base-content"
          : "text-base-content-muted hover:bg-base-content/[0.06] hover:text-base-content"
      }`}
      href="/settings"
      aria-current={activeKey === "settings" ? "page" : undefined}
      title={$t("header.settings")}
      aria-label={$t("header.settings")}
    >
      <Settings
        size={17}
        strokeWidth={1.7}
        class="transition-transform duration-500 ease-out hover:rotate-90"
      />
    </a>
  </div>
</header>
