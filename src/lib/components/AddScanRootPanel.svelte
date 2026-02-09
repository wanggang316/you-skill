<script>
  import { Plus, X } from "@lucide/svelte";
  import IconButton from "./ui/IconButton.svelte";
  import { t } from "../i18n";

  let {
    newScanRoot = $bindable(),
    localError,
    scanRoots,
    onAddRoot,
    onRemoveRoot,
  } = $props();
</script>

<section class="space-y-6">
  <div
    class="rounded-2xl border border-base-300 bg-base-100 p-4"
  >
    <p class="mb-3 text-sm font-semibold text-base-content-muted">
      {$t("view.add.title")}
    </p>
    <div class="flex flex-wrap items-center gap-3">
      <input
        class="flex-1 rounded-xl border border-base-300 bg-base-200 px-4 py-2 text-sm text-base-content placeholder:text-base-content-subtle"
        placeholder={$t("view.add.placeholder")}
        bind:value={newScanRoot}
      />
      <IconButton
        variant="primary"
        onclick={onAddRoot}
        title={$t("view.add.addPath")}
        ariaLabel={$t("view.add.addPath")}
      >
        <Plus size={16} />
      </IconButton>
    </div>
    {#if localError}
      <p class="mt-3 text-sm text-error">{localError}</p>
    {/if}
    {#if scanRoots.length > 0}
      <div class="mt-3 space-y-2 text-sm text-base-content-muted">
        {#each scanRoots as root}
          <div
            class="flex items-center justify-between rounded-lg bg-base-200 px-3 py-2"
          >
            <span>{root}</span>
            <IconButton
              variant="ghost"
              class="text-error"
              onclick={() => onRemoveRoot(root)}
              title={$t("view.add.removePath")}
              ariaLabel={$t("view.add.removePath")}
            >
              <X size={14} />
            </IconButton>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</section>
