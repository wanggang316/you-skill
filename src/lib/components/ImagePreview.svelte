<script lang="ts">
  let { src = "", alt = "" }: { src?: string; alt?: string } = $props();
</script>

<div class="flex justify-center py-2">
  <img src={src} alt={alt} class="h-auto max-h-[75vh] max-w-full object-contain" />
</div>
