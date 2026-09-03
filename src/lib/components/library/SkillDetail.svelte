<script lang="ts">
  import { ExternalLink, FileText, FolderOpen, Loader2, RefreshCw, Trash2 } from "@lucide/svelte";
  import { open as openExternal } from "@tauri-apps/plugin-shell";
  import IconButton from "$lib/components/ui/IconButton.svelte";
  import PrimaryActionButton from "$lib/components/ui/PrimaryActionButton.svelte";
  import MarkdownPreview from "$lib/components/MarkdownPreview.svelte";
  import InstallMatrix, { type TargetAction } from "./InstallMatrix.svelte";
  import DriftPanel from "./DriftPanel.svelte";
  import { t } from "$lib/i18n";
  import { parseMarkdown, renderMarkdownBody } from "$lib/utils/markdown";
  import { readSkillFile, type AgentInfo } from "$lib/api/skills";
  import {
    sourceLabel,
    sourceRepoUrl,
    type HubSkillView,
    type InstallScope,
    type InstallView,
    type SyncAction,
  } from "$lib/api/hub";
  import type { UserProject } from "$lib/api/user-projects";

  let {
    skill,
    projects = [],
    agents,
    busy = false,
    checkingSource = false,
    actionError = "",
    onInstall,
    onTargetAction,
    onSync,
    onRemove,
    onOpenDir,
    onCheckSource,
    onViewFiles,
  }: {
    skill: HubSkillView;
    projects?: UserProject[];
    agents: Map<string, AgentInfo>;
    busy?: boolean;
    checkingSource?: boolean;
    actionError?: string;
    onInstall: (scope: InstallScope, projectPath: string | null) => void;
    onTargetAction: (install: InstallView, action: TargetAction, agentId: string) => void;
    onSync: (action: SyncAction) => void;
    onRemove: () => void;
    onOpenDir: () => void;
    onCheckSource: () => void;
    onViewFiles: () => void;
  } = $props();

  let readmeLoading = $state(false);
  let readmeError = $state("");
  let readmeHtml = $state("");
  let readmeDescription = $state("");
  let readmeFor = $state("");

  const repoUrl = $derived(sourceRepoUrl(skill.source));
  const shortHash = $derived(skill.hash.slice(0, 10));
  const formatDate = (value: string) => {
    const date = new Date(value);
    return Number.isNaN(date.getTime()) ? value : date.toLocaleString();
  };

  $effect(() => {
    const key = `${skill.name}|${skill.hubHash ?? ""}`;
    if (readmeFor === key) return;
    readmeFor = key;
    loadReadme();
  });

  async function loadReadme() {
    readmeLoading = true;
    readmeError = "";
    try {
      const markdown = await readSkillFile(skill.hubPath);
      const parsed = parseMarkdown(markdown);
      readmeHtml = renderMarkdownBody(parsed.content);
      readmeDescription = parsed.hasFrontmatter
        ? ((parsed.frontmatter as Record<string, string>).description ?? "")
        : "";
    } catch (error) {
      readmeHtml = "";
      readmeDescription = "";
      readmeError = String(error);
    } finally {
      readmeLoading = false;
    }
  }

  function handleTargetPush(install: InstallView) {
    onTargetAction(install, "push", install.agentIds[0] ?? "");
  }

  function handleTargetAdopt(install: InstallView) {
    onTargetAction(install, "adopt", install.agentIds[0] ?? "");
  }
</script>

<div class="flex min-h-0 min-w-0 flex-col">
  <header
    class="border-base-300 flex h-12 flex-none items-center justify-between gap-3 border-b px-6"
    data-window-drag-region
  >
    <div class="flex min-w-0 items-center gap-2">
      <h2 class="text-base-content truncate text-[1.05rem] font-semibold tracking-[-0.02em]">
        {skill.name}
      </h2>
      {#if skill.hasDrift}
        <span class="tag tag-warning">{$t("library.tag.changed")}</span>
      {/if}
    </div>
    <div class="flex shrink-0 items-center gap-1.5">
      <IconButton
        variant="outline"
        onclick={onViewFiles}
        title={$t("detail.viewFiles")}
        class="h-8 w-8 p-0"
      >
        <FileText size={15} />
      </IconButton>
      <IconButton
        variant="outline"
        onclick={onOpenDir}
        title={$t("detail.openDir")}
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

  <div class="min-h-0 flex-1 overflow-y-auto">
    <div class="mx-auto max-w-4xl space-y-6 px-6 py-5">
      <dl
        class="text-base-content-muted grid grid-cols-[auto_minmax(0,1fr)] gap-x-4 gap-y-1.5 text-xs"
      >
        <dt>{$t("detail.source")}</dt>
        <dd class="flex min-w-0 items-center gap-2">
          <span class="tag tag-neutral shrink-0">{$t(`detail.source.${skill.source.type}`)}</span>
          {#if skill.source.type === "github" && repoUrl}
            <button
              class="text-primary inline-flex min-w-0 items-center gap-1 truncate hover:underline"
              type="button"
              onclick={() => openExternal(repoUrl)}
              title={repoUrl}
            >
              <span class="truncate">{sourceLabel(skill.source)}</span>
              <ExternalLink size={11} class="shrink-0" />
            </button>
          {:else if sourceLabel(skill.source)}
            <span class="truncate" title={sourceLabel(skill.source)}
              >{sourceLabel(skill.source)}</span
            >
          {/if}
          {#if skill.source.type === "github" || skill.source.type === "folder"}
            <button
              class="text-base-content-subtle hover:text-base-content inline-flex shrink-0 items-center gap-1 text-[11px]"
              type="button"
              onclick={onCheckSource}
              disabled={checkingSource || busy}
              title={$t("detail.checkSource")}
            >
              {#if checkingSource}
                <Loader2 size={11} class="animate-spin" />
              {:else}
                <RefreshCw size={11} />
              {/if}
              {checkingSource ? $t("detail.checking") : $t("detail.checkSource")}
            </button>
          {/if}
        </dd>
        <dt>{$t("detail.hash")}</dt>
        <dd class="font-mono" title={skill.hash}>
          {shortHash}
          {#if skill.hubHash && skill.hubHash !== skill.hash}
            <span class="text-warning-content"> → {skill.hubHash.slice(0, 10)}</span>
          {/if}
        </dd>
        <dt>{$t("detail.importedAt")}</dt>
        <dd>{formatDate(skill.importedAt)}</dd>
        <dt>{$t("detail.updatedAt")}</dt>
        <dd>{formatDate(skill.updatedAt)}</dd>
      </dl>

      {#if actionError}
        <p class="text-error text-sm whitespace-pre-wrap">{actionError}</p>
      {/if}

      <section class="space-y-2">
        <h3 class="text-base-content text-sm font-medium">{$t("detail.installs")}</h3>
        <InstallMatrix {skill} {projects} {agents} {busy} onAdd={onInstall} {onTargetAction} />
        {#if skill.installs.length === 0}
          <p class="text-base-content-faint text-xs">{$t("detail.installs.empty")}</p>
        {/if}
      </section>

      {#if skill.hasDrift}
        <section class="space-y-2">
          <h3 class="text-base-content text-sm font-medium">{$t("detail.drift")}</h3>
          <DriftPanel
            {skill}
            {busy}
            {onSync}
            onTargetPush={handleTargetPush}
            onTargetAdopt={handleTargetAdopt}
          />
        </section>
      {/if}

      <section class="space-y-2">
        <h3 class="text-base-content text-sm font-medium">{$t("detail.readme")}</h3>
        {#if readmeLoading}
          <div class="text-base-content-muted flex items-center gap-2 py-4 text-xs">
            <Loader2 size={14} class="animate-spin" />
          </div>
        {:else if readmeError}
          <p class="text-error text-xs">{$t("detail.readmeError")}: {readmeError}</p>
        {:else}
          <div class="border-base-300 rounded-2xl border p-5">
            <MarkdownPreview
              htmlContent={readmeHtml}
              frontmatterDescription={readmeDescription}
              onOpenExternalLink={(href) => openExternal(href)}
              onOpenRelativeLink={() => onViewFiles()}
            />
          </div>
        {/if}
      </section>
    </div>
  </div>
</div>
