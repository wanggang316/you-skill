<script lang="ts">
  let { class: className = "" }: { class?: string } = $props();
</script>

<!--
  Full-window overlays cover the app top bar. This strip sits over the top bar area
  so the window stays draggable while the overlay is open. Its height matches AppTopBar.
-->
<div
  class={`fixed inset-x-0 top-0 h-15 ${className}`}
  data-window-drag-region
  aria-hidden="true"
></div>
