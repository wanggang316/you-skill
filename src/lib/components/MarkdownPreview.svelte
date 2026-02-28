<script lang="ts">
  import type { Action } from "svelte/action";

  let {
    htmlContent = "",
    interactionAction,
  }: {
    htmlContent?: string;
    interactionAction?: Action<HTMLElement, undefined>;
  } = $props();
</script>

{#if interactionAction}
  <div class="markdown-content" use:interactionAction>
    {@html htmlContent}
  </div>
{:else}
  <div class="markdown-content">
    {@html htmlContent}
  </div>
{/if}
