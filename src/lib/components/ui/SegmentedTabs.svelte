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
    className = "",
  } = $props<{
    items?: SegmentedTabItem[];
    value?: string;
    onChange?: (value: string) => void;
    fullWidth?: boolean;
    className?: string;
  }>();
</script>

<div class={`bg-base-200 flex gap-2 rounded-full p-1 ${className}`}>
  {#each items as item}
    <button
      class={`rounded-full px-4 py-2 text-sm transition hover:text-base-content ${
        fullWidth ? "flex flex-1 items-center justify-center gap-2" : ""
      } ${
        value === item.value
          ? "bg-base-100 text-base-content shadow-sm"
          : "text-base-content-muted"
      }`}
      onclick={() => onChange(item.value)}
      type="button"
    >
      {#if item.icon}
        {@const Icon = item.icon}
        <Icon size={16} />
      {/if}
      {item.label}
    </button>
  {/each}
</div>
