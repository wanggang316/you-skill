<script lang="ts">
  import {
    ExternalLink,
    FolderOpen,
    Loader2,
    PackageMinus,
    RefreshCw,
    Trash2,
  } from "@lucide/svelte";
  import { open as openExternal } from "@tauri-apps/plugin-shell";
  import IconButton from "$lib/components/ui/IconButton.svelte";
  import PrimaryActionButton from "$lib/components/ui/PrimaryActionButton.svelte";
  import SegmentedTabs from "$lib/components/ui/SegmentedTabs.svelte";
  import SkillFileViewer from "$lib/components/SkillFileViewer.svelte";
  import InstallTargets from "./InstallTargets.svelte";
  import DriftPanel, { type TargetAction } from "./DriftPanel.svelte";
  import { t } from "$lib/i18n";
  import type { AgentInfo } from "$lib/api/skills";
  import {
    scopedHasDrift,
    scopedInstalls,
    sourceLabel,
    sourceRepoUrl,
    type HubSkillView,
    type InstallScope,
    type InstallView,
    type ScopeRef,
    type SyncAction,
  } from "$lib/api/hub";
  import type { UserProject } from "$lib/api/user-projects";

  type DetailTab = "general" | "files";

  let {
    skill,
    projects = [],
    agents,
    busy = false,
    checkingSource = false,
    actionError = "",
    scope = null,
    onInstall,
    onTargetAction,
    onSync,
    onRemove,
    onUninstallScope,
    onOpenDir,
    onCheckSource,
  }: {
    skill: HubSkillView;
    projects?: UserProject[];
    agents: Map<string, AgentInfo>;
    busy?: boolean;
    checkingSource?: boolean;
    actionError?: string;
    /** When set, only this scope's installs are shown and actions target it. */
    scope?: ScopeRef | null;
    onInstall: (scope: InstallScope, projectPath: string | null) => void;
    onTargetAction: (install: InstallView, action: TargetAction) => void;
    onSync: (action: SyncAction) => void;
    onRemove: () => void;
    /** Uninstall from the current scope (scoped view only). */
    onUninstallScope?: () => void;
    onOpenDir: () => void;
    onCheckSource: () => void;
  } = $props();

  let tab = $state<DetailTab>("general");

  const scopeInstalls = $derived(scope ? scopedInstalls(skill, scope) : skill.installs);
  const panelSkill = $derived<HubSkillView>(scope ? { ...skill, installs: scopeInstalls } : skill);
  const hasDrift = $derived(scope ? scopedHasDrift(skill, scope) : skill.hasDrift);
  const repoUrl = $derived(sourceRepoUrl(skill.source));
  const shortHash = $derived(skill.hash.slice(0, 10));
  const formatDate = (value: string) => {
    const date = new Date(value);
    return Number.isNaN(date.getTime()) ? value : date.toLocaleString();
  };
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
      {#if hasDrift}
        <span class="tag tag-warning">{$t("library.tag.changed")}</span>
      {/if}
    </div>
    <div class="flex shrink-0 items-center gap-1.5">
      <IconButton
        variant="outline"
        onclick={onOpenDir}
        title={$t("detail.openDir")}
        class="h-8 w-8 p-0"
      >
        <FolderOpen size={15} />
      </IconButton>
      {#if scope}
        <IconButton
          variant="outline"
          onclick={() => onUninstallScope?.()}
          title={$t("scope.uninstall")}
          class="text-error h-8 w-8 p-0"
          disabled={busy || scopeInstalls.length === 0}
        >
          <PackageMinus size={15} />
        </IconButton>
      {:else}
        <IconButton
          variant="outline"
          onclick={onRemove}
          title={$t("detail.remove")}
          class="text-error h-8 w-8 p-0"
          disabled={busy}
        >
          <Trash2 size={15} />
        </IconButton>
      {/if}
      <PrimaryActionButton
        onclick={() =>
          scope ? onInstall(scope.scope, scope.projectPath) : onInstall("user", null)}
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
        { value: "general", label: $t("detail.tab.general") },
        { value: "files", label: $t("detail.tab.files") },
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

        {#if hasDrift}
          <section class="space-y-2">
            <h3 class="text-base-content text-sm font-medium">{$t("detail.drift")}</h3>
            <DriftPanel skill={panelSkill} {busy} {onSync} {onTargetAction} />
          </section>
        {/if}

        <section class="space-y-2">
          <h3 class="text-base-content text-sm font-medium">
            {scope ? $t("scope.agents") : $t("detail.installs")}
          </h3>
          <InstallTargets
            skill={panelSkill}
            {projects}
            {agents}
            {busy}
            only={scope}
            onAdd={onInstall}
          />
          {#if !scope && skill.installs.length === 0}
            <p class="text-base-content-faint text-xs">{$t("detail.installs.empty")}</p>
          {/if}
        </section>
      </div>
    </div>
  {:else}
    {#key skill.hubPath}
      <SkillFileViewer source={{ kind: "hub", name: skill.name, rootPath: skill.hubPath }} dense />
    {/key}
  {/if}
</div>
