<script lang="ts">
  import { MoreHorizontal } from "@lucide/svelte";

  export type MenuItem = {
    label: string;
    danger?: boolean;
    disabled?: boolean;
    onSelect: () => void;
  };

  let {
    items = [],
    label = "",
    disabled = false,
  }: {
    items?: MenuItem[];
    label?: string;
    disabled?: boolean;
  } = $props();

  let open = $state(false);
  let root = $state<HTMLDivElement>();

  $effect(() => {
    if (!open) return;
    const onPointerDown = (event: MouseEvent) => {
      if (root && event.target instanceof Node && !root.contains(event.target)) open = false;
    };
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") open = false;
    };
    document.addEventListener("mousedown", onPointerDown);
    document.addEventListener("keydown", onKeyDown);
    return () => {
      document.removeEventListener("mousedown", onPointerDown);
      document.removeEventListener("keydown", onKeyDown);
    };
  });
</script>

<div class="relative" bind:this={root}>
  <button
    class="border-base-300 text-base-content-muted hover:bg-base-200 hover:text-base-content flex size-7 items-center justify-center rounded-lg border transition disabled:opacity-40"
    type="button"
    title={label}
    aria-label={label}
    aria-haspopup="menu"
    aria-expanded={open}
    disabled={disabled || items.length === 0}
    onclick={(event) => {
      event.stopPropagation();
      open = !open;
    }}
  >
    <MoreHorizontal size={15} />
  </button>
  {#if open}
    <div
      class="border-base-300 bg-base-100 absolute top-full right-0 z-30 mt-1 min-w-44 rounded-xl border p-1 shadow-lg"
      role="menu"
    >
      {#each items as item (item.label)}
        <button
          class={`w-full rounded-lg px-2.5 py-1.5 text-left text-[13px] transition disabled:opacity-40 ${
            item.danger ? "text-error hover:bg-error/10" : "text-base-content hover:bg-base-200"
          }`}
          type="button"
          role="menuitem"
          disabled={item.disabled}
          onclick={(event) => {
            event.stopPropagation();
            open = false;
            item.onSelect();
          }}
        >
          {item.label}
        </button>
      {/each}
    </div>
  {/if}
</div>
