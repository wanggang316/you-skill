<script lang="ts">
  let { renderedCode = "", lineNumbersText = "" }: { renderedCode?: string; lineNumbersText?: string } =
    $props();
</script>

<div class="code-preview-shell">
  <pre class="code-preview-gutter" aria-hidden="true">{lineNumbersText}</pre>
  <pre class="code-preview-plain"><code class="hljs">{@html renderedCode}</code></pre>
</div>

<style>
  .code-preview-shell {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    align-items: start;
    padding: 0;
    overflow-x: auto;
    overflow-y: hidden;
    scrollbar-gutter: stable;
  }

  .code-preview-gutter {
    margin: 0;
    padding: 0;
    padding-right: 0.5rem;
    font-family: "Fira Code", "Monaco", "Consolas", "Courier New", monospace;
    font-size: 0.75rem;
    line-height: 1.6;
    text-align: right;
    color: var(--base-content-faint);
    white-space: pre;
    -webkit-user-select: none;
    user-select: none;
    pointer-events: none;
    border-right: 0.5px solid color-mix(in oklch, var(--base-content) 6%, transparent);
  }

  .code-preview-plain {
    margin: 0;
    padding: 0 0 0 0.5rem;
    font-family: "Fira Code", "Monaco", "Consolas", "Courier New", monospace;
    font-size: 0.75rem;
    line-height: 1.6;
    color: var(--base-content);
    white-space: pre;
  }

  .code-preview-plain code {
    display: block;
    font: inherit;
    line-height: inherit;
    white-space: inherit;
    background: transparent;
    color: inherit;
    margin: 0;
    padding: 0;
  }

  .code-preview-shell::-webkit-scrollbar {
    height: 8px;
  }

  .code-preview-shell::-webkit-scrollbar-track {
    background: color-mix(in oklch, var(--base-content) 3%, transparent);
  }

  .code-preview-shell::-webkit-scrollbar-thumb {
    background: color-mix(in oklch, var(--base-content) 14%, transparent);
    border-radius: 9999px;
  }
</style>
