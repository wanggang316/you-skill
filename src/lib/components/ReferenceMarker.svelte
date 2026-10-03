<script lang="ts">
  import { AtSign } from "@lucide/svelte";
  import { t } from "$lib/i18n";

  let {
    references = [],
    size = 12,
  }: {
    /** Files a pure reference file imports (`@AGENTS.md`); the marker is hidden when empty. */
    references?: string[];
    size?: number;
  } = $props();

  const label = $derived($t("reference.target", { path: references.join("\n") }));
</script>

{#if references.length > 0}
  <span
    class="text-base-content-subtle inline-flex shrink-0 items-center"
    role="img"
    title={label}
    aria-label={label}
  >
    <AtSign {size} />
  </span>
{/if}
