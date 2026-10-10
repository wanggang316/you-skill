<script lang="ts">
  import { onMount } from "svelte";
  import { Loader2, Settings, Zap } from "@lucide/svelte";
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
    onOpenLibrary,
    onOpenUpdate,
  }: {
    activeKey: SidebarActiveKey;
    hasUpdate: boolean;
    updateLoading: boolean;
    latestVersion?: string;
    onOpenLibrary: () => void;
    onOpenUpdate: () => void;
  } = $props();

  type NavItem = {
    key: SidebarActiveKey;
    href: string;
    label: string;
    onclick?: () => void;
  };

  /** Local content first, then the marketplace as its own group. */
  const groups: NavItem[][] = $derived([
    [
      {
        key: "library",
        href: buildLibraryHref(),
        label: $t("sidebar.library"),
        onclick: onOpenLibrary,
      },
      { key: "instructions", href: buildInstructionsHref(), label: $t("sidebar.instructions") },
      { key: "projects", href: buildScopeHref(), label: $t("sidebar.projects") },
    ],
    [{ key: "market", href: buildMarketHref(), label: $t("sidebar.market") }],
  ]);

  let nav = $state<HTMLElement | null>(null);
  /** Position of the active pill; it slides between items instead of jumping. */
  let indicator = $state({ left: 0, width: 0, visible: false });
  /** No slide on first paint, only on later moves. */
  let animate = $state(false);

  function measure() {
    const active = nav?.querySelector<HTMLElement>(`[data-nav-key="${activeKey}"]`);
    indicator = active
      ? { left: active.offsetLeft, width: active.offsetWidth, visible: true }
      : { ...indicator, visible: false };
  }

  $effect(() => {
    // Re-measure when the route or the labels (locale) change.
    void activeKey;
    void groups;
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
  class="bg-base-100 grid h-12 flex-none grid-cols-[1fr_auto_1fr] items-center gap-4 pr-3 pl-[5.25rem]"
  data-window-drag-region
>
  <a
    class="group focus-visible:ring-primary/50 flex w-fit min-w-0 items-center gap-2 rounded-lg focus-visible:ring-2 focus-visible:outline-none"
    href={buildLibraryHref()}
    onclick={onOpenLibrary}
    aria-label="YouSkill"
  >
    <span
      class="from-base-content to-base-content/75 text-base-100 ring-base-content/10 flex size-[22px] flex-none items-center justify-center rounded-[7px] bg-gradient-to-b shadow-[0_1px_2px_0_var(--color-overlay)] ring-1 transition-transform duration-200 group-hover:-rotate-6"
      aria-hidden="true"
    >
      <Zap size={12} strokeWidth={2.4} class="fill-current" />
    </span>
    <span class="text-base-content truncate text-[14px] font-semibold tracking-[-0.025em] italic">
      YouSkill
    </span>
  </a>

  <nav
    bind:this={nav}
    class="bg-base-200 border-base-300 relative flex h-8 items-center rounded-full border p-[3px] shadow-[inset_0_1px_1px_0_var(--color-base-300)]"
    aria-label={$t("sidebar.navigation")}
  >
    <span
      class={`bg-base-content absolute top-[3px] bottom-[3px] left-0 rounded-full shadow-[0_1px_3px_0_var(--color-overlay)] ${
        animate
          ? "transition-[transform,width,opacity] duration-300 ease-[cubic-bezier(0.32,0.72,0,1)]"
          : ""
      }`}
      style={`transform: translateX(${indicator.left}px); width: ${indicator.width}px; opacity: ${
        indicator.visible ? 1 : 0
      };`}
      aria-hidden="true"
    ></span>
    {#each groups as group, index}
      {#if index > 0}
        <span class="bg-base-content/12 mx-1.5 h-3.5 w-px" aria-hidden="true"></span>
      {/if}
      {#each group as item (item.key)}
        {@const active = activeKey === item.key}
        <a
          class={`focus-visible:ring-primary/50 relative z-10 flex h-full items-center rounded-full px-3.5 text-[12.5px] leading-none font-medium tracking-[-0.005em] whitespace-nowrap transition-colors duration-200 focus-visible:ring-2 focus-visible:outline-none ${
            active ? "text-base-100" : "text-base-content-muted hover:text-base-content"
          }`}
          href={item.href}
          onclick={item.onclick}
          data-nav-key={item.key}
          aria-current={active ? "page" : undefined}
        >
          {item.label}
        </a>
      {/each}
    {/each}
  </nav>

  <div class="flex min-w-0 items-center justify-end gap-1.5">
    {#if hasUpdate}
      <button
        class="border-primary/25 bg-primary/8 text-primary hover:bg-primary/14 flex h-7 min-w-0 items-center gap-1.5 rounded-full border px-2.5 text-[12px] font-medium transition-colors duration-150 disabled:cursor-default disabled:opacity-60"
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
        <span class="truncate">
          {$t("header.update")}{latestVersion ? ` · v${latestVersion}` : ""}
        </span>
      </button>
    {/if}
    <a
      class={`flex size-8 items-center justify-center rounded-full transition-[background-color,color,transform] duration-200 active:scale-95 ${
        activeKey === "settings"
          ? "bg-base-200 text-base-content"
          : "text-base-content-muted hover:bg-base-200 hover:text-base-content"
      }`}
      href="/settings"
      aria-current={activeKey === "settings" ? "page" : undefined}
      title={$t("header.settings")}
      aria-label={$t("header.settings")}
    >
      <Settings
        size={16}
        strokeWidth={1.7}
        class="transition-transform duration-500 ease-out hover:rotate-90"
      />
    </a>
  </div>
</header>
