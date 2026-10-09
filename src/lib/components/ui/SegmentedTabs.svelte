<script lang="ts">
  import type { Component } from "svelte";

  export type SegmentedTabItem = {
    value: string;
    label: string;
    icon?: Component<{ size?: number }>;
  };

  let {
    items = [],
    value = "",
    onChange = () => {},
    fullWidth = false,
    size = "md",
    className = "",
  } = $props<{
    items?: SegmentedTabItem[];
    value?: string;
    onChange?: (value: string) => void;
    fullWidth?: boolean;
    size?: "md" | "sm";
    className?: string;
  }>();
</script>

<div
  class={`bg-base-200 flex rounded-full ${size === "sm" ? "gap-0.5 p-0.5" : "gap-2 p-1"} ${className}`}
>
  {#each items as item}
    <button
      class={`hover:text-base-content rounded-full transition ${
        size === "sm" ? "px-2.5 py-1 text-xs" : "px-4 py-2 text-sm"
      } ${fullWidth ? "flex flex-1 items-center justify-center gap-2" : ""} ${
        value === item.value ? "bg-base-100 text-base-content shadow-sm" : "text-base-content-muted"
      }`}
      onclick={() => onChange(item.value)}
      type="button"
    >
      {#if item.icon}
        {@const Icon = item.icon}
        <Icon size={size === "sm" ? 13 : 16} />
      {/if}
      {item.label}
    </button>
  {/each}
</div>
