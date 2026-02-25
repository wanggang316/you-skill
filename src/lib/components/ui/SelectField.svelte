<script lang="ts">
  import { ChevronsUpDown } from "@lucide/svelte";
  import type { Snippet } from "svelte";
  import type { HTMLSelectAttributes } from "svelte/elements";

  let {
    value = $bindable(),
    disabled = false,
    className = "",
    selectClassName = "",
    children,
    ...restProps
  } = $props<
    HTMLSelectAttributes & {
      className?: string;
      selectClassName?: string;
      children?: Snippet;
    }
  >();

  const baseSelectClass =
    "border-base-300 bg-base-100 text-base-content focus:border-base-300 h-9 cursor-pointer appearance-none rounded-xl border pr-8 pl-3 text-sm focus:outline-none disabled:cursor-not-allowed disabled:opacity-60";
</script>

<div class={`relative ${className}`}>
  <select
    {...restProps}
    bind:value
    {disabled}
    class={`${baseSelectClass} ${selectClassName}`.trim()}
  >
    {#if children}
      {@render children()}
    {/if}
  </select>
  <ChevronsUpDown
    class="text-base-content-subtle pointer-events-none absolute top-1/2 right-3 -translate-y-1/2"
    size={14}
  />
</div>
