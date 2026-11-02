<script lang="ts">
  import { Bookmark } from "@lucide/svelte";
  import SegmentedTabs, { type SegmentedTabItem } from "$lib/components/ui/SegmentedTabs.svelte";

  let {
    tabs,
    tab,
    favorites,
    favoritesLabel,
    onSelectTab,
    onShowFavorites,
  }: {
    tabs: SegmentedTabItem[];
    tab: string;
    /** The favorites view shows instead of a tab's list. */
    favorites: boolean;
    favoritesLabel: string;
    onSelectTab: (value: string) => void;
    onShowFavorites: () => void;
  } = $props();
</script>

<div class="flex items-center gap-2">
  <SegmentedTabs
    items={tabs}
    className="min-w-0 flex-1 [&>button]:min-w-0 [&>button]:px-2 [&>button]:text-xs [&>button]:whitespace-nowrap"
    value={favorites ? "" : tab}
    onChange={onSelectTab}
    fullWidth
    surface="sidebar"
  />
  <button
    type="button"
    class="favorites-button"
    class:active={favorites}
    title={favoritesLabel}
    aria-label={favoritesLabel}
    aria-pressed={favorites}
    onclick={onShowFavorites}
    data-favorites-button
  >
    <Bookmark size={16} fill={favorites ? "currentColor" : "none"} aria-hidden="true" />
  </button>
</div>

<style>
  .favorites-button {
    display: inline-flex;
    flex-shrink: 0;
    align-items: center;
    justify-content: center;
    width: 2.5rem;
    height: 2.5rem;
    border-radius: 9999px;
    background: color-mix(in oklch, var(--color-base-content) 5%, transparent);
    color: var(--color-base-content-muted);
    transition:
      background-color 200ms,
      color 200ms;
  }
  .favorites-button:hover {
    color: var(--color-base-content);
  }
  .favorites-button.active {
    background: var(--color-raised);
    color: var(--color-base-content);
    box-shadow: 0 1px 2px color-mix(in oklch, var(--color-base-content) 12%, transparent);
  }
  .favorites-button:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
  }
</style>
