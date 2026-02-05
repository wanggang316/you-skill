<script>
  import { Plus, X } from '@lucide/svelte'

  let {
    newScanRoot = $bindable(),
    localError,
    scanRoots,
    onAddRoot,
    onRemoveRoot
  } = $props()
</script>

<section class="space-y-6">
  <div class="rounded-2xl border border-[var(--base-300)] bg-[var(--base-100)] p-4">
    <p class="mb-3 text-sm font-semibold text-[var(--base-content-muted)]">
      新增扫描路径
    </p>
    <div class="flex flex-wrap items-center gap-3">
      <input
        class="flex-1 rounded-xl border border-[var(--base-300)] bg-[var(--base-200)] px-4 py-2 text-sm text-[var(--base-content)] placeholder:text-[var(--base-content-subtle)]"
        placeholder="添加自定义扫描路径（项目根目录）"
        bind:value={newScanRoot}
      />
      <button
        class="rounded-xl bg-[var(--primary)] p-2 text-sm text-[var(--primary-content)]"
        onclick={onAddRoot}
        title="添加路径"
        type="button"
      >
        <Plus size={16} />
      </button>
    </div>
    {#if localError}
      <p class="mt-3 text-sm text-[var(--error)]">{localError}</p>
    {/if}
    {#if scanRoots.length > 0}
      <div class="mt-3 space-y-2 text-sm text-[var(--base-content-muted)]">
        {#each scanRoots as root}
          <div
            class="flex items-center justify-between rounded-lg bg-[var(--base-200)] px-3 py-2"
          >
            <span>{root}</span>
            <button
              class="text-[var(--error)]"
              onclick={() => onRemoveRoot(root)}
              title="移除路径"
              type="button"
            >
              <X size={14} />
            </button>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</section>
