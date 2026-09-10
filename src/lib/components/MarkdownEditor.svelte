<script lang="ts">
  import { Loader2, Pencil } from "@lucide/svelte";
  import { open as openExternal } from "@tauri-apps/plugin-shell";
  import MarkdownPreview from "$lib/components/MarkdownPreview.svelte";
  import PrimaryActionButton from "$lib/components/ui/PrimaryActionButton.svelte";
  import { t } from "$lib/i18n";
  import { renderMarkdownBody } from "$lib/utils/markdown";

  let {
    content = "",
    loading = false,
    error = "",
    disabled = false,
    onSave,
  }: {
    content?: string;
    loading?: boolean;
    /** Why the content could not be loaded. */
    error?: string;
    disabled?: boolean;
    /** Persist the text; the returned strings are places that were not updated. */
    onSave: (text: string) => Promise<string[] | void>;
  } = $props();

  let editing = $state(false);
  let draft = $state("");
  let saving = $state(false);
  let saveError = $state("");
  let blockers = $state<string[]>([]);

  const html = $derived(renderMarkdownBody(content));

  function startEdit() {
    draft = content;
    saveError = "";
    blockers = [];
    editing = true;
  }

  function cancelEdit() {
    editing = false;
    draft = "";
  }

  async function save() {
    if (saving) return;
    saving = true;
    saveError = "";
    try {
      blockers = (await onSave(draft)) ?? [];
      editing = false;
    } catch (err) {
      saveError = String(err);
    } finally {
      saving = false;
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    if ((event.metaKey || event.ctrlKey) && event.key === "s") {
      event.preventDefault();
      void save();
    }
  }

  const buttonClass =
    "border-base-300 text-base-content hover:bg-base-200 flex h-7 items-center gap-1 rounded-lg border px-2.5 text-xs transition disabled:opacity-50";
</script>

<div class="space-y-3">
  <div class="flex items-center justify-end gap-1.5">
    {#if editing}
      <button class={buttonClass} type="button" onclick={cancelEdit} disabled={saving}>
        {$t("common.cancel")}
      </button>
      <PrimaryActionButton
        onclick={save}
        className="h-7 px-3 py-0 text-xs"
        disabled={saving}
        loading={saving}
      >
        {$t("instructions.save")}
      </PrimaryActionButton>
    {:else}
      <button
        class={buttonClass}
        type="button"
        onclick={startEdit}
        disabled={disabled || loading || Boolean(error)}
      >
        <Pencil size={12} />
        <span>{$t("instructions.edit")}</span>
      </button>
    {/if}
  </div>

  {#if saveError}
    <p class="text-error text-sm whitespace-pre-wrap">{saveError}</p>
  {/if}
  {#if blockers.length > 0}
    <div class="border-warning/40 bg-warning/5 rounded-2xl border p-3 text-xs">
      <p class="text-base-content">{$t("instructions.saveBlocked")}</p>
      <ul class="text-base-content-muted mt-1 space-y-0.5">
        {#each blockers as blocker}
          <li class="truncate" title={blocker}>{blocker}</li>
        {/each}
      </ul>
    </div>
  {/if}

  {#if editing}
    <textarea
      class="border-base-300 bg-base-200 text-base-content focus:border-primary min-h-[60vh] w-full resize-y rounded-xl border px-4 py-3 font-mono text-[13px] leading-relaxed focus:outline-none"
      bind:value={draft}
      onkeydown={handleKeydown}
      disabled={saving}
      spellcheck="false"
    ></textarea>
  {:else if loading}
    <div class="text-base-content-muted flex items-center gap-2 py-10 text-sm">
      <Loader2 size={16} class="animate-spin" />
    </div>
  {:else if error}
    <p class="text-error text-sm whitespace-pre-wrap">
      {$t("instructions.contentError")}: {error}
    </p>
  {:else}
    <MarkdownPreview htmlContent={html} onOpenExternalLink={(href) => openExternal(href)} />
  {/if}
</div>
