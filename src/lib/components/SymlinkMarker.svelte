<script lang="ts">
  import { Link2, Link2Off } from "@lucide/svelte";
  import { t } from "$lib/i18n";

  let {
    target = null,
    broken = false,
    size = 12,
  }: {
    /** Path the entry links to; the marker is hidden when the entry is not a symlink. */
    target?: string | null;
    /** The link target does not exist. */
    broken?: boolean;
    size?: number;
  } = $props();

  const label = $derived(
    $t(broken ? "symlink.brokenTarget" : "symlink.target", { path: target ?? "" })
  );
</script>

{#if target}
  <span
    class="inline-flex shrink-0 items-center {broken ? 'text-error' : 'text-base-content-subtle'}"
    role="img"
    title={label}
    aria-label={label}
  >
    {#if broken}
      <Link2Off {size} />
    {:else}
      <Link2 {size} />
    {/if}
  </span>
{/if}
