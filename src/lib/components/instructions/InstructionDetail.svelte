<script lang="ts">
  import { untrack } from "svelte";
  import { ExternalLink, FolderOpen, Loader2, Pencil, Plus, Trash2 } from "@lucide/svelte";
  import { open as openExternal } from "@tauri-apps/plugin-shell";
  import IconButton from "$lib/components/ui/IconButton.svelte";
  import PrimaryActionButton from "$lib/components/ui/PrimaryActionButton.svelte";
  import SegmentedTabs from "$lib/components/ui/SegmentedTabs.svelte";
  import MarkdownPreview from "$lib/components/MarkdownPreview.svelte";
  import InstallTargets from "$lib/components/library/InstallTargets.svelte";
  import DriftPanel, { type TargetAction } from "$lib/components/library/DriftPanel.svelte";
  import { t } from "$lib/i18n";
  import {
    instructionSourceLabel,
    instructionSourceUrl,
    readInstruction,
    writeInstruction,
    type InstructionView,
  } from "$lib/api/instructions";
  import { openInFileManager } from "$lib/api/skills";
  import { applyInstructionView, refreshInstructions } from "$lib/stores/instructions";
  import type { InstallScope, InstallView, SyncAction } from "$lib/api/hub";
  import type { AgentInfo } from "$lib/api/skills";
  import type { UserProject } from "$lib/api/user-projects";
  import { renderMarkdownBody } from "$lib/utils/markdown";

  type DetailTab = "general" | "content";

  let {
    instruction,
    projects = [],
    agents,
    homePath = "",
    busy = false,
    actionError = "",
    onInstall,
    onManageLocations,
    onTargetAction,
    onSync,
    onRemove,
    onOpenFile,
  }: {
    instruction: InstructionView;
    projects?: UserProject[];
    agents: Map<string, AgentInfo>;
    homePath?: string;
    busy?: boolean;
    actionError?: string;
    onInstall: (scope: InstallScope, projectPath: string | null, lockScope?: boolean) => void;
    onManageLocations: () => void;
    onTargetAction: (install: InstallView, action: TargetAction) => void;
    onSync: (action: SyncAction) => void;
    onRemove: () => void;
    onOpenFile: () => void;
  } = $props();

  let tab = $state<DetailTab>("general");
  let content = $state("");
  let contentError = $state("");
  let contentLoading = $state(false);
  let loadedKey = $state("");
  let editing = $state(false);
  let draft = $state("");
  let saving = $state(false);
  let saveError = $state("");
  /** Copy targets with local changes that the last save left alone. */
  let saveBlockers = $state<string[]>([]);

  const shortHash = $derived(instruction.hash.slice(0, 10));
  const sourceLabel = $derived(instructionSourceLabel(instruction.source));
  const sourceUrl = $derived(instructionSourceUrl(instruction.source));
  /** Local sources open in the file manager, GitHub ones in the browser. */
  const sourceHref = $derived.by(() => {
    if (sourceUrl) return sourceUrl;
    const source = instruction.source;
    return source.type === "file" || source.type === "agent" ? source.path : null;
  });
  function openSource() {
    if (sourceUrl) void openExternal(sourceUrl);
    else if (sourceHref) openInFileManager(sourceHref).catch(console.error);
  }
  const html = $derived(renderMarkdownBody(content));
  const formatDate = (value: string) => {
    const date = new Date(value);
    return Number.isNaN(date.getTime()) ? value : date.toLocaleString();
  };

  // Read the file when the content tab is shown, again whenever it changed on disk.
  $effect(() => {
    if (tab !== "content") return;
    const key = `${instruction.name}|${instruction.hubHash ?? instruction.hash}`;
    if (key === untrack(() => loadedKey)) return;
    loadedKey = key;
    contentLoading = true;
    contentError = "";
    readInstruction(instruction.name)
      .then((text) => (content = text))
      .catch((error) => (contentError = String(error)))
      .finally(() => (contentLoading = false));
  });

  function startEdit() {
    draft = content;
    saveError = "";
    saveBlockers = [];
    editing = true;
  }

  function cancelEdit() {
    editing = false;
    draft = "";
  }

  /** Save the library file; unmodified copies follow, edited ones are listed. */
  async function save() {
    if (saving) return;
    saving = true;
    saveError = "";
    try {
      const result = await writeInstruction(instruction.name, draft);
      const view = result.instruction;
      if (view) {
        // Keep the editor's text instead of re-reading the file it was just written to.
        loadedKey = `${view.name}|${view.hubHash ?? view.hash}`;
        content = draft === "" || draft.endsWith("\n") ? draft : `${draft}\n`;
        applyInstructionView(view);
      }
      saveBlockers = result.blockers;
      editing = false;
      await refreshInstructions();
    } catch (error) {
      saveError = String(error);
    } finally {
      saving = false;
    }
  }

  function handleEditorKeydown(event: KeyboardEvent) {
    if ((event.metaKey || event.ctrlKey) && event.key === "s") {
      event.preventDefault();
      void save();
    }
  }

  const toolbarButtonClass =
    "border-base-300 text-base-content hover:bg-base-200 flex h-7 items-center gap-1 rounded-lg border px-2.5 text-xs transition disabled:opacity-50";
</script>

<div class="flex min-h-0 min-w-0 flex-col">
  <header
    class="border-base-300 flex h-12 flex-none items-center justify-between gap-3 border-b px-6"
    data-window-drag-region
  >
    <div class="flex min-w-0 items-center gap-2">
      <h2 class="text-base-content truncate text-[1.05rem] font-semibold tracking-[-0.02em]">
        {instruction.name}
      </h2>
      {#if instruction.hasDrift}
        <span class="tag tag-warning">{$t("library.tag.changed")}</span>
      {/if}
    </div>
    <div class="flex shrink-0 items-center gap-1.5">
      <IconButton
        variant="outline"
        onclick={onOpenFile}
        title={$t("instructions.openFile")}
        class="h-8 w-8 p-0"
      >
        <FolderOpen size={15} />
      </IconButton>
      <IconButton
        variant="outline"
        onclick={onRemove}
        title={$t("detail.remove")}
        class="text-error h-8 w-8 p-0"
        disabled={busy}
      >
        <Trash2 size={15} />
      </IconButton>
      <PrimaryActionButton
        onclick={() => onInstall("user", null)}
        className="h-8 px-3 py-0 text-[13px]"
        disabled={busy}
      >
        {$t("detail.install")}
      </PrimaryActionButton>
    </div>
  </header>

  <div class="border-base-300 flex flex-none items-center border-b px-6 py-2">
    <SegmentedTabs
      items={[
        { value: "general", label: $t("instructions.tab.general") },
        { value: "content", label: $t("instructions.tab.content") },
      ]}
      value={tab}
      onChange={(value) => (tab = value as DetailTab)}
      className="text-xs"
    />
  </div>

  {#if tab === "general"}
    <div class="min-h-0 flex-1 overflow-y-auto">
      <div class="mx-auto max-w-4xl space-y-6 px-6 py-5">
        <dl
          class="text-base-content-muted grid grid-cols-[auto_minmax(0,1fr)] gap-x-4 gap-y-1.5 text-xs"
        >
          <dt>{$t("detail.source")}</dt>
          <dd class="flex min-w-0 items-center gap-2">
            <span class="tag tag-neutral shrink-0">
              {$t(`instructions.source.${instruction.source.type}`)}
            </span>
            {#if sourceHref}
              <button
                class="text-primary inline-flex min-w-0 items-center gap-1 truncate hover:underline"
                type="button"
                onclick={openSource}
                title={sourceHref}
              >
                <span class="truncate font-mono">{sourceLabel}</span>
                {#if sourceUrl}
                  <ExternalLink size={11} class="shrink-0" />
                {/if}
              </button>
            {/if}
          </dd>
          <dt>{$t("instructions.file")}</dt>
          <dd class="truncate font-mono" title={instruction.hubPath}>{instruction.hubPath}</dd>
          <dt>{$t("detail.hash")}</dt>
          <dd class="font-mono" title={instruction.hash}>
            {shortHash}
            {#if instruction.hubHash && instruction.hubHash !== instruction.hash}
              <span class="text-warning-content"> → {instruction.hubHash.slice(0, 10)}</span>
            {/if}
          </dd>
          <dt>{$t("detail.importedAt")}</dt>
          <dd>{formatDate(instruction.importedAt)}</dd>
          <dt>{$t("detail.updatedAt")}</dt>
          <dd>{formatDate(instruction.updatedAt)}</dd>
        </dl>

        {#if actionError}
          <p class="text-error text-sm whitespace-pre-wrap">{actionError}</p>
        {/if}

        {#if instruction.hasDrift}
          <section class="space-y-2">
            <h3 class="text-base-content text-sm font-medium">{$t("detail.drift")}</h3>
            <DriftPanel
              name={instruction.name}
              kind="instruction"
              hubState={instruction.hubState}
              installs={instruction.installs}
              {busy}
              {onSync}
              {onTargetAction}
            />
          </section>
        {/if}

        <section class="space-y-2">
          <div class="flex items-center justify-between gap-3">
            <h3 class="text-base-content text-sm font-medium">{$t("detail.installs")}</h3>
            <button
              class="border-base-300 text-base-content-muted hover:border-primary hover:text-primary flex h-7 items-center gap-1 rounded-lg border border-dashed px-2 text-[12px] transition disabled:opacity-50"
              type="button"
              onclick={onManageLocations}
              disabled={busy}
            >
              <Plus size={13} />
              <span>{$t("detail.installs.addLocation")}</span>
            </button>
          </div>
          <InstallTargets
            installs={instruction.installs}
            {projects}
            {agents}
            {homePath}
            {busy}
            onAdd={onInstall}
          />
          {#if instruction.installs.length === 0}
            <p class="text-base-content-faint text-xs">{$t("instructions.installs.empty")}</p>
          {/if}
        </section>
      </div>
    </div>
  {:else}
    <div class="min-h-0 flex-1 overflow-y-auto">
      <div class="mx-auto max-w-4xl space-y-3 px-6 py-5">
        <div class="flex items-center justify-end gap-1.5">
          {#if editing}
            <button class={toolbarButtonClass} type="button" onclick={cancelEdit} disabled={saving}>
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
              class={toolbarButtonClass}
              type="button"
              onclick={startEdit}
              disabled={busy || contentLoading || Boolean(contentError)}
            >
              <Pencil size={12} />
              <span>{$t("instructions.edit")}</span>
            </button>
          {/if}
        </div>

        {#if saveError}
          <p class="text-error text-sm whitespace-pre-wrap">{saveError}</p>
        {/if}
        {#if saveBlockers.length > 0}
          <div class="border-warning/40 bg-warning/5 rounded-2xl border p-3 text-xs">
            <p class="text-base-content">{$t("instructions.saveBlocked")}</p>
            <ul class="text-base-content-muted mt-1 space-y-0.5">
              {#each saveBlockers as blocker}
                <li class="truncate" title={blocker}>{blocker}</li>
              {/each}
            </ul>
          </div>
        {/if}

        {#if editing}
          <textarea
            class="border-base-300 bg-base-200 text-base-content focus:border-primary min-h-[60vh] w-full resize-y rounded-xl border px-4 py-3 font-mono text-[13px] leading-relaxed focus:outline-none"
            bind:value={draft}
            onkeydown={handleEditorKeydown}
            disabled={saving}
            spellcheck="false"
          ></textarea>
        {:else if contentLoading}
          <div class="text-base-content-muted flex items-center gap-2 py-10 text-sm">
            <Loader2 size={16} class="animate-spin" />
          </div>
        {:else if contentError}
          <p class="text-error text-sm whitespace-pre-wrap">
            {$t("instructions.contentError")}: {contentError}
          </p>
        {:else}
          <MarkdownPreview htmlContent={html} onOpenExternalLink={(href) => openExternal(href)} />
        {/if}
      </div>
    </div>
  {/if}
</div>
