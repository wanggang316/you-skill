<script lang="ts">
  import { ChevronsUpDown } from "@lucide/svelte";
  import type { Snippet } from "svelte";
  import type { HTMLSelectAttributes } from "svelte/elements";

  let {
    value = $bindable(),
    disabled = false,
    fit = false,
    className = "",
    selectClassName = "",
    children,
    ...restProps
  } = $props<
    HTMLSelectAttributes & {
      /** Size to the selected option. A native select is as wide as its longest option. */
      fit?: boolean;
      className?: string;
      selectClassName?: string;
      children?: Snippet;
    }
  >();

  const baseSelectClass =
    "border-base-300 bg-base-100 text-base-content focus:border-base-300 h-9 cursor-pointer appearance-none rounded-xl border pr-8 pl-3 text-sm focus:outline-none disabled:cursor-not-allowed disabled:opacity-60";

  let selectEl = $state<HTMLSelectElement>();
  let selectedLabel = $state("");

  function readSelectedLabel() {
    selectedLabel = selectEl?.selectedOptions[0]?.textContent?.trim() ?? "";
  }

  $effect(() => {
    if (!fit || !selectEl) return;
    void value;
    readSelectedLabel();
    // Option labels change with the UI language without a change of value.
    const observer = new MutationObserver(readSelectedLabel);
    observer.observe(selectEl, { childList: true, subtree: true, characterData: true });
    return () => observer.disconnect();
  });
</script>

<div class={`relative ${className}`}>
  {#if fit}
    <!-- Invisible copy of the selected label that sets the width; the select overlays it. -->
    <span
      class={`invisible block h-9 border pr-8 pl-3 text-sm whitespace-pre ${selectClassName}`.trim()}
      aria-hidden="true">{selectedLabel}</span
    >
  {/if}
  <select
    {...restProps}
    bind:this={selectEl}
    bind:value
    {disabled}
    onchange={(event) => {
      readSelectedLabel();
      restProps.onchange?.(event);
    }}
    class={`${baseSelectClass} ${fit ? "absolute inset-0 w-full" : "w-full"} ${selectClassName}`.trim()}
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
