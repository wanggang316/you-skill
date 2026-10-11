<script lang="ts">
  import type { Snippet } from "svelte";

  /** One line of a `.settings-card`: label and detail on the left, controls on the right. */
  let {
    label,
    description = "",
    children,
  }: {
    label: string;
    description?: string;
    children?: Snippet;
  } = $props();
</script>

<div class="flex min-h-14 items-center justify-between gap-6 px-4 py-3">
  <div class="min-w-0">
    <div class="text-base-content text-[13.5px]">{label}</div>
    {#if description}
      <div class="text-base-content-muted mt-0.5 text-xs break-all">{description}</div>
    {/if}
  </div>
  {#if children}
    <div class="flex shrink-0 items-center gap-2">{@render children()}</div>
  {/if}
</div>
