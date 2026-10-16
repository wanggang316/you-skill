<script lang="ts">
  import FrontmatterValue from "./FrontmatterValue.svelte";
  import type { FrontmatterNode } from "$lib/utils/frontmatter";

  let {
    node,
    onOpenExternalLink,
  }: {
    node: FrontmatterNode;
    onOpenExternalLink?: (href: string) => Promise<void> | void;
  } = $props();

  const tokenClass =
    "border-base-300 bg-base-100 text-base-content inline-block max-w-full rounded-md border px-1.5 py-0.5 font-mono text-xs break-all";
</script>

{#if node.kind === "text"}
  <span class="break-words whitespace-pre-wrap">{node.value}</span>
{:else if node.kind === "link"}
  <button
    class="text-primary hover:text-primary-hover text-left break-all underline underline-offset-2"
    type="button"
    onclick={() => onOpenExternalLink?.(node.href)}
  >
    {node.href}
  </button>
{:else if node.kind === "literal"}
  <span class="font-mono">{node.value}</span>
{:else if node.kind === "empty"}
  <span class="text-base-content-faint font-mono">null</span>
{:else if node.kind === "tokens"}
  <span class="flex flex-wrap gap-1.5">
    {#each node.values as value, index (index)}
      <code class={tokenClass}>{value}</code>
    {/each}
  </span>
{:else if node.kind === "list"}
  <ul class="flex flex-col gap-1.5">
    {#each node.items as item, index (index)}
      <li class="flex gap-2">
        <span class="text-base-content-faint select-none">-</span>
        <div class="min-w-0 flex-1">
          <FrontmatterValue node={item} {onOpenExternalLink} />
        </div>
      </li>
    {/each}
  </ul>
{:else if node.kind === "map"}
  <!-- The key column takes the widest key so the values line up. -->
  <dl class="grid grid-cols-[max-content_minmax(0,1fr)] gap-x-3 gap-y-1.5">
    {#each node.entries as entry (entry.key)}
      <dt class="text-base-content-muted font-mono text-xs leading-6">{entry.key}:</dt>
      <dd class="min-w-0">
        <FrontmatterValue node={entry.node} {onOpenExternalLink} />
      </dd>
    {/each}
  </dl>
{/if}
