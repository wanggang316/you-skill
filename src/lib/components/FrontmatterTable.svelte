<script lang="ts">
  import FrontmatterValue from "./FrontmatterValue.svelte";
  import { toFrontmatterEntries } from "$lib/utils/frontmatter";

  let {
    frontmatter,
    translatedDescription = "",
    onOpenExternalLink,
  }: {
    frontmatter: Record<string, unknown>;
    /** Shown under the `description` value in bilingual view. */
    translatedDescription?: string;
    onOpenExternalLink?: (href: string) => Promise<void> | void;
  } = $props();

  const entries = $derived(toFrontmatterEntries(frontmatter));
</script>

{#if entries.length}
  <div class="border-base-300 mb-6 overflow-hidden rounded-xl border">
    <table class="text-base-content w-full border-collapse text-sm leading-6">
      <tbody>
        {#each entries as entry, index (entry.key)}
          <tr
            class="border-base-300 border-t first:border-t-0"
            class:bg-base-100={index % 2 === 0}
            class:bg-base-200={index % 2 === 1}
          >
            <th
              class="border-base-300 w-44 border-r px-4 py-3 text-center align-middle font-semibold break-words"
              scope="row"
            >
              {entry.key}
            </th>
            <td class="px-4 py-3 align-middle">
              <FrontmatterValue node={entry.node} {onOpenExternalLink} />
              {#if entry.key === "description" && translatedDescription}
                <div class="markdown-translation mt-1 break-words whitespace-pre-wrap">
                  {translatedDescription}
                </div>
              {/if}
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
{/if}
