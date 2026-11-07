<script lang="ts">
  import { CheckCircle2 } from "@lucide/svelte";
  import { fly } from "svelte/transition";
  import { dismissToast, toasts } from "$lib/stores/toast";
</script>

<div
  class="pointer-events-none fixed right-4 bottom-4 z-(--z-popover) flex flex-col items-end gap-2"
  role="status"
  aria-live="polite"
>
  {#each $toasts as toast (toast.id)}
    <button
      class="border-base-300 bg-base-100 text-base-content pointer-events-auto flex max-w-sm items-center gap-2 rounded-xl border px-3 py-2 text-left text-[13px] shadow-lg"
      type="button"
      onclick={() => dismissToast(toast.id)}
      transition:fly={{ y: 8, duration: 150 }}
    >
      <CheckCircle2 size={15} class="text-success shrink-0" />
      <span class="min-w-0">{toast.message}</span>
    </button>
  {/each}
</div>
