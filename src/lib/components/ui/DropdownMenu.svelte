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

  const MENU_GAP = 4;
  const VIEWPORT_MARGIN = 8;

  let open = $state(false);
  let trigger = $state<HTMLButtonElement>();
  let menu = $state<HTMLDivElement>();
  let position = $state<{ top: number; left: number } | null>(null);

  const close = () => {
    open = false;
    position = null;
  };

  // Place the menu after it renders, so its real size decides the flip and the clamp.
  $effect(() => {
    if (!open || !trigger || !menu) return;
    const anchor = trigger.getBoundingClientRect();
    const { width, height } = menu.getBoundingClientRect();
    const maxTop = window.innerHeight - VIEWPORT_MARGIN - height;
    const below = anchor.bottom + MENU_GAP;
    const above = anchor.top - MENU_GAP - height;
    const top = below <= maxTop || above < VIEWPORT_MARGIN ? below : above;
    position = {
      top: Math.max(VIEWPORT_MARGIN, Math.min(top, maxTop)),
      left: Math.max(
        VIEWPORT_MARGIN,
        Math.min(anchor.right - width, window.innerWidth - VIEWPORT_MARGIN - width)
      ),
    };
  });

  $effect(() => {
    if (!open) return;
    const onPointerDown = (event: MouseEvent) => {
      if (!(event.target instanceof Node)) return;
      if (trigger?.contains(event.target) || menu?.contains(event.target)) return;
      close();
    };
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") close();
    };
    // The menu is fixed to the viewport, so any scroll or resize would leave it behind its trigger.
    document.addEventListener("mousedown", onPointerDown);
    document.addEventListener("keydown", onKeyDown);
    window.addEventListener("scroll", close, true);
    window.addEventListener("resize", close);
    return () => {
      document.removeEventListener("mousedown", onPointerDown);
      document.removeEventListener("keydown", onKeyDown);
      window.removeEventListener("scroll", close, true);
      window.removeEventListener("resize", close);
    };
  });

  /** Render the menu under <body> so overflow clipping and modal transforms cannot affect it. */
  function portal(node: HTMLElement) {
    document.body.appendChild(node);
    return {
      destroy() {
        node.remove();
      },
    };
  }
</script>

<div class="relative">
  <button
    bind:this={trigger}
    class="border-base-300 text-base-content-muted hover:bg-base-200 hover:text-base-content flex size-7 items-center justify-center rounded-lg border transition disabled:opacity-40"
    type="button"
    title={label}
    aria-label={label}
    aria-haspopup="menu"
    aria-expanded={open}
    disabled={disabled || items.length === 0}
    onclick={(event) => {
      event.stopPropagation();
      if (open) close();
      else open = true;
    }}
  >
    <MoreHorizontal size={15} />
  </button>
  {#if open}
    <div
      use:portal
      bind:this={menu}
      class="border-base-300 bg-base-100 fixed z-[10020] min-w-28 rounded-xl border p-1 shadow-lg"
      style={position
        ? `top:${position.top}px; left:${position.left}px;`
        : "top:0; left:0; visibility:hidden;"}
      role="menu"
    >
      {#each items as item (item.label)}
        <button
          class={`w-full rounded-lg px-2.5 py-1.5 text-left text-[13px] whitespace-nowrap transition disabled:opacity-40 ${
            item.danger ? "text-error hover:bg-error/10" : "text-base-content hover:bg-base-200"
          }`}
          type="button"
          role="menuitem"
          disabled={item.disabled}
          onclick={(event) => {
            event.stopPropagation();
            close();
            item.onSelect();
          }}
        >
          {item.label}
        </button>
      {/each}
    </div>
  {/if}
</div>
