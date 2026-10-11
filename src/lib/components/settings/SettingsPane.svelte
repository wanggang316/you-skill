<script lang="ts">
  import type { Snippet } from "svelte";

  /** Scrolling body of one settings section: title row, short description, then the content. */
  let {
    title,
    description = "",
    actions,
    children,
  }: {
    title: string;
    description?: string;
    actions?: Snippet;
    children: Snippet;
  } = $props();
</script>

<div
  class="min-h-0 overflow-y-auto [scrollbar-color:var(--scrollbar-thumb)_transparent] [scrollbar-width:thin]"
>
  <div class="mx-auto w-full max-w-3xl px-8 pt-8 pb-12">
    <div class="flex items-center justify-between gap-4">
      <h2 class="text-base-content truncate text-[1.25rem] font-semibold tracking-[-0.02em]">
        {title}
      </h2>
      {#if actions}
        <div class="flex shrink-0 items-center gap-1.5">{@render actions()}</div>
      {/if}
    </div>
    {#if description}
      <p class="text-base-content-muted mt-1 text-[13px] leading-relaxed">{description}</p>
    {/if}
    <div class="mt-6 space-y-6">
      {@render children()}
    </div>
  </div>
</div>
