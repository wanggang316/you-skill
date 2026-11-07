<script lang="ts">
  import { MoreHorizontal } from "@lucide/svelte";
  import { DropdownMenu } from "bits-ui";
  import { menuContentClass, menuItemClass, type MenuItem } from "./menu";

  const defaultTriggerClass =
    "border-base-300 text-base-content-muted hover:bg-base-200 hover:text-base-content data-[state=open]:bg-base-200 flex size-7 items-center justify-center rounded-lg border transition disabled:opacity-40";

  let {
    items = [],
    label = "",
    disabled = false,
    triggerClass = defaultTriggerClass,
    iconSize = 15,
  }: {
    items?: MenuItem[];
    label?: string;
    disabled?: boolean;
    /** Replaces the default outlined trigger style. */
    triggerClass?: string;
    iconSize?: number;
  } = $props();
</script>

<DropdownMenu.Root>
  <DropdownMenu.Trigger
    class={triggerClass}
    title={label}
    aria-label={label}
    disabled={disabled || items.length === 0}
    onclick={(event) => event.stopPropagation()}
  >
    <MoreHorizontal size={iconSize} />
  </DropdownMenu.Trigger>
  <DropdownMenu.Portal>
    <DropdownMenu.Content class={menuContentClass} align="end" sideOffset={4} collisionPadding={8}>
      {#each items as item (item.label)}
        <DropdownMenu.Item
          class={menuItemClass(item.danger)}
          disabled={item.disabled}
          onSelect={item.onSelect}
        >
          {item.label}
        </DropdownMenu.Item>
      {/each}
    </DropdownMenu.Content>
  </DropdownMenu.Portal>
</DropdownMenu.Root>
