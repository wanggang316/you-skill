<script lang="ts">
  import { Github } from "@lucide/svelte";
  import { cachedGithubAvatar, loadGithubAvatar } from "$lib/github-avatar";

  let { owner }: { owner: string } = $props();

  let url = $state<string | null | undefined>(undefined);

  $effect(() => {
    const current = owner;
    const cached = cachedGithubAvatar(current);
    url = cached;
    if (cached !== undefined) return;
    let active = true;
    loadGithubAvatar(current).then((result) => {
      if (active) url = result;
    });
    return () => {
      active = false;
    };
  });
</script>

<!-- The GitHub icon stands in while loading and when the avatar is not available. -->
{#if url}
  <img class="size-4 shrink-0 rounded-full" src={url} alt="" onerror={() => (url = null)} />
{:else}
  <Github size={13} class="shrink-0" />
{/if}
