<script>
  import { Plus, X } from "@lucide/svelte";
  import IconButton from "./ui/IconButton.svelte";
  import { t } from "../i18n";

  let { newScanRoot = $bindable(), localError, scanRoots, onAddRoot, onRemoveRoot } = $props();
</script>

<section class="space-y-6">
  <div class="border-base-300 bg-base-100 rounded-2xl border p-4">
    <p class="text-base-content-muted mb-3 text-sm font-semibold">
      {$t("view.add.title")}
    </p>
    <div class="flex flex-wrap items-center gap-3">
      <input
        class="border-base-300 bg-base-200 text-base-content placeholder:text-base-content-subtle flex-1 rounded-xl border px-4 py-2 text-sm"
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
      <p class="text-error mt-3 text-sm">{localError}</p>
    {/if}
    {#if scanRoots.length > 0}
      <div class="text-base-content-muted mt-3 space-y-2 text-sm">
        {#each scanRoots as root}
          <div class="bg-base-200 flex items-center justify-between rounded-lg px-3 py-2">
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
