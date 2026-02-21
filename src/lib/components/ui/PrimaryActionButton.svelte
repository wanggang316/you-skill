<script lang="ts">
  import { Loader2 } from "@lucide/svelte";
  import type { Snippet } from "svelte";
  import type { HTMLButtonAttributes } from "svelte/elements";

  let {
    disabled = false,
    loading = false,
    className = "",
    type = "button",
    loadingText = "",
    children,
    ...restProps
  } = $props<
    HTMLButtonAttributes & {
      loading?: boolean;
      className?: string;
      loadingText?: string;
      children?: Snippet;
    }
  >();
</script>

<button
  {...restProps}
  class={`bg-primary text-primary-content hover:bg-primary-hover disabled:bg-primary/60 inline-flex items-center justify-center gap-2 rounded-xl px-4 py-2 text-sm transition ${className}`}
  {type}
  disabled={disabled || loading}
>
  {#if loading}
    <Loader2 size={16} class="animate-spin" />
    {#if loadingText}
      <span>{loadingText}</span>
    {/if}
  {:else if children}
    {@render children()}
  {/if}
</button>
