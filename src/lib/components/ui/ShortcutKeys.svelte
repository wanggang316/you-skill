<script lang="ts">
  import { bindingKeys, type Binding } from "$lib/shortcuts";

  /** One key cap per key of a binding, e.g. ⌘ and , for "Mod+,". */
  let { binding }: { binding: Binding } = $props();

  const keys = $derived(bindingKeys(binding));
</script>

<span class="inline-flex items-center gap-px" aria-hidden="true">
  {#each keys as key, index (index)}
    <kbd
      class="text-base-content-faint inline-flex min-w-[0.6em] items-center justify-center font-sans text-[12px] leading-none font-medium"
    >
      {key}
    </kbd>
  {/each}
</span>
