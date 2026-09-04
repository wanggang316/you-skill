<script lang="ts">
  import { FileArchive, Folder, Github, Zap } from "@lucide/svelte";
  import type { SkillSource } from "$lib/api/hub";

  let { source, size = 13 }: { source: SkillSource; size?: number } = $props();
</script>

{#if source.type === "github"}
  <Github {size} />
{:else if source.type === "zip"}
  <FileArchive {size} />
{:else if source.type === "folder"}
  <Folder {size} />
{:else}
  <Zap {size} />
{/if}
