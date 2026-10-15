<script lang="ts">
  import { bindingKeys, type Binding } from "$lib/shortcuts";

  /** One key cap per key of a binding, e.g. ⌘ and , for "Mod+,". */
  let { binding }: { binding: Binding } = $props();

  const keys = $derived(bindingKeys(binding));
</script>

<span class="inline-flex items-center gap-0.5" aria-hidden="true">
  {#each keys as key, index (index)}
    <kbd
      class="border-base-content/10 bg-base-content/[0.06] text-base-content-muted inline-flex h-[18px] min-w-[18px] items-center justify-center rounded-[5px] border px-1 font-sans text-[11px] leading-none font-medium"
    >
      {key}
    </kbd>
  {/each}
</span>
