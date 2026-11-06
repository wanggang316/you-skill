<script lang="ts">
  import DropdownMenu from "$lib/components/ui/DropdownMenu.svelte";
  import type { MenuItem } from "$lib/components/ui/menu";

  /** The `...` menu of a Library group header; each group kind gives its own items. */
  let {
    items,
    label,
    disabled = false,
  }: {
    items: MenuItem[];
    label: string;
    disabled?: boolean;
  } = $props();
</script>

<DropdownMenu
  {items}
  {label}
  {disabled}
  iconSize={13}
  triggerClass="text-base-content-subtle hover:bg-base-content/[0.07] hover:text-base-content data-[state=open]:bg-base-content/[0.07] data-[state=open]:text-base-content focus-visible:ring-primary/40 flex size-5 shrink-0 items-center justify-center rounded-md transition outline-none focus-visible:ring-2 disabled:opacity-40"
/>
