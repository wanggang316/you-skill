<script lang="ts">
  import type { Snippet } from "svelte";
  import { ContextMenu } from "bits-ui";
  import { menuContentClass, menuItemClass, type MenuItem } from "./menu";

  let {
    items = [],
    trigger,
  }: {
    items?: MenuItem[];
    /** Renders the element that opens the menu on right click; spread `props` onto it. */
    trigger: Snippet<[{ props: Record<string, unknown> }]>;
  } = $props();
</script>

<ContextMenu.Root>
  <ContextMenu.Trigger disabled={items.length === 0}>
    {#snippet child({ props })}
      {@render trigger({ props })}
    {/snippet}
  </ContextMenu.Trigger>
  <ContextMenu.Portal>
    <ContextMenu.Content class={menuContentClass} collisionPadding={8}>
      {#each items as item (item.label)}
        <ContextMenu.Item
          class={menuItemClass(item.danger)}
          disabled={item.disabled}
          onSelect={item.onSelect}
        >
          {item.label}
        </ContextMenu.Item>
      {/each}
    </ContextMenu.Content>
  </ContextMenu.Portal>
</ContextMenu.Root>
