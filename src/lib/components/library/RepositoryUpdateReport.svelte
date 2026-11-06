<script lang="ts">
  import { AlertCircle, SkipForward } from "@lucide/svelte";
  import Modal from "$lib/components/ui/Modal.svelte";
  import { t } from "$lib/i18n";
  import type { GroupUpdateReport, SkipReason } from "$lib/repository-actions";
  import { skillLabel } from "$lib/stores/hub";

  let {
    open = $bindable(false),
    repo = "",
    report = null,
  }: {
    open?: boolean;
    /** `owner/repo` of the group. */
    repo?: string;
    report?: GroupUpdateReport | null;
  } = $props();

  const skipReasonKeys: Record<SkipReason, string> = {
    local_changes: "library.repoUpdate.skippedReason",
    changed_after_confirmation: "library.repoUpdate.changedAfterConfirmation",
    changed_after_check: "library.repoUpdate.changedAfterCheck",
  };

  /** A removed skill has no label any more; fall back to the name of the report. */
  const labelOf = (item: { id: string; name: string }) =>
    $skillLabel(item.id) === item.id ? item.name : $skillLabel(item.id);
</script>

<Modal
  bind:open
  title={$t("library.repoUpdate.title", { repo })}
  containerClass="max-w-md"
  closeOnBackdropClick={true}
>
  {#if report}
    <div class="space-y-4 px-6 pt-2 pb-6 text-sm">
      <p class="text-base-content">
        {$t("library.repoUpdate.summary", {
          updated: report.updated,
          upToDate: report.upToDate,
        })}
      </p>
      {#if report.skipped.length > 0}
        <section class="space-y-1.5">
          <h3 class="text-base-content-muted flex items-center gap-1.5 text-xs font-medium">
            <SkipForward size={13} class="text-warning shrink-0" />
            {$t("library.repoUpdate.skipped", { count: report.skipped.length })}
          </h3>
          <ul class="bg-base-200 max-h-40 space-y-1 overflow-y-auto rounded-xl p-3 text-xs">
            {#each report.skipped as item (item.id)}
              <li class="flex items-baseline justify-between gap-3">
                <span class="text-base-content max-w-[45%] shrink-0 truncate">
                  {labelOf(item)}
                </span>
                <span class="text-base-content-muted min-w-0 text-right">
                  {$t(skipReasonKeys[item.reason])}
                </span>
              </li>
            {/each}
          </ul>
        </section>
      {/if}
      {#if report.failed.length > 0}
        <section class="space-y-1.5">
          <h3 class="text-base-content-muted flex items-center gap-1.5 text-xs font-medium">
            <AlertCircle size={13} class="text-error shrink-0" />
            {$t("library.repoUpdate.failed", { count: report.failed.length })}
          </h3>
          <ul class="bg-base-200 max-h-48 space-y-2 overflow-y-auto rounded-xl p-3 text-xs">
            {#each report.failed as item (item.id)}
              <li class="space-y-0.5">
                <p class="text-base-content truncate">{labelOf(item)}</p>
                <p class="text-error break-words whitespace-pre-wrap">{item.error}</p>
              </li>
            {/each}
          </ul>
        </section>
      {/if}
    </div>
  {/if}
  {#snippet footer()}
    <button
      class="border-base-300 text-base-content hover:bg-base-200 rounded-xl border px-4 py-2 text-sm transition"
      type="button"
      onclick={() => (open = false)}
    >
      {$t("common.close")}
    </button>
  {/snippet}
</Modal>
