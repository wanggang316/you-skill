<script lang="ts">
  import {
    AlertTriangle,
    CheckCircle2,
    FileText,
    FolderOpen,
    Plus,
    ScanSearch,
    X,
  } from "@lucide/svelte";
  import AgentStack from "$lib/components/AgentStack.svelte";
  import SkillIcon from "$lib/components/SkillIcon.svelte";
  import DropdownMenu, { type MenuItem } from "$lib/components/ui/DropdownMenu.svelte";
  import IconButton from "$lib/components/ui/IconButton.svelte";
  import { t } from "$lib/i18n";
  import { scopedInstalls, type HubSkillView, type InstallView } from "$lib/api/hub";
  import type { MemoryFile } from "$lib/api/agent-apps";
  import type { AgentInfo } from "$lib/api/skills";
  import type { ScopeEntry } from "$lib/scopes";

  export type ScopeSkillAction = "manage" | "diff" | "push" | "adopt" | "openDir" | "uninstall";

  let {
    entry,
    agents,
    availableAgents = [],
    memoryFiles = [],
    busy = false,
    actionError = "",
    onOpenDir,
    onScan,
    onAddSkills,
    onAddAgent,
    onRemoveAgents,
    onOpenMemory,
    onOpenSkill,
    onSkillAction,
  }: {
    entry: ScopeEntry;
    agents: Map<string, AgentInfo>;
    /** Agents that can still be added to this scope. */
    availableAgents?: AgentInfo[];
    memoryFiles?: MemoryFile[];
    busy?: boolean;
    actionError?: string;
    onOpenDir: () => void;
    onScan: () => void;
    onAddSkills: () => void;
    onAddAgent: () => void;
    onRemoveAgents: (agentIds: string[]) => void;
    onOpenMemory: (file: MemoryFile) => void;
    onOpenSkill: (name: string) => void;
    onSkillAction: (skill: HubSkillView, action: ScopeSkillAction) => void;
  } = $props();

  type AgentGroup = { key: string; path: string; agentIds: string[]; skillCount: number };

  /** Agents that read the same directory are one entry: they are installed together. */
  const agentGroups = $derived.by((): AgentGroup[] => {
    const map = new Map<string, { agentIds: string[]; skills: Set<string> }>();
    for (const skill of entry.skills) {
      for (const install of scopedInstalls(skill, entry.ref)) {
        const group = map.get(install.path) ?? { agentIds: [], skills: new Set<string>() };
        for (const id of install.agentIds) {
          if (!group.agentIds.includes(id)) group.agentIds.push(id);
        }
        group.skills.add(skill.name);
        map.set(install.path, group);
      }
    }
    return [...map.entries()].map(([path, group]) => ({
      key: path,
      path,
      agentIds: group.agentIds,
      skillCount: group.skills.size,
    }));
  });

  const presentMemory = $derived(memoryFiles.filter((file) => file.exists));
  const canAddAgent = $derived(availableAgents.length > 0 && entry.skills.length > 0);

  function installsOf(skill: HubSkillView): InstallView[] {
    return scopedInstalls(skill, entry.ref);
  }

  function relativePath(path: string): string {
    if (entry.path && path.startsWith(entry.path)) {
      return path.slice(entry.path.length).replace(/^[/\\]+/, "");
    }
    return path;
  }

  function locationLabel(installs: InstallView[]): string {
    if (installs.length !== 1) return $t("scope.skill.locations", { count: installs.length });
    return relativePath(installs[0].path);
  }

  function skillMenu(skill: HubSkillView): MenuItem[] {
    const installs = installsOf(skill);
    const drifted = installs.filter((install) => install.state !== "in_sync");
    const diffable = drifted.find((install) => install.mode === "copy");
    const adoptable = drifted.find(
      (install) => install.state === "modified" || install.state === "conflict"
    );
    const items: MenuItem[] = [
      { label: $t("scope.skill.manage"), onSelect: () => onSkillAction(skill, "manage") },
    ];
    if (diffable) {
      items.push({ label: $t("diff.view"), onSelect: () => onSkillAction(skill, "diff") });
    }
    if (drifted.length > 0) {
      items.push({ label: $t("target.push"), onSelect: () => onSkillAction(skill, "push") });
    }
    if (adoptable) {
      items.push({ label: $t("target.adopt"), onSelect: () => onSkillAction(skill, "adopt") });
    }
    items.push({ label: $t("detail.openDir"), onSelect: () => onSkillAction(skill, "openDir") });
    items.push({
      label: $t("scope.uninstall"),
      danger: true,
      onSelect: () => onSkillAction(skill, "uninstall"),
    });
    return items;
  }
</script>

<div class="flex min-h-0 min-w-0 flex-col">
  <header
    class="border-base-300 flex h-12 flex-none items-center justify-between gap-3 border-b px-6"
    data-window-drag-region
  >
    <div class="flex min-w-0 items-center gap-2">
      <h2 class="text-base-content truncate text-[1.05rem] font-semibold tracking-[-0.02em]">
        {entry.name}
      </h2>
      {#if entry.unregistered}
        <span class="tag tag-neutral">{$t("projects.unregistered")}</span>
      {/if}
    </div>
    <div class="flex shrink-0 items-center gap-1.5">
      {#if entry.path}
        <IconButton
          variant="outline"
          onclick={onOpenDir}
          title={$t("detail.openDir")}
          class="h-8 w-8 p-0"
        >
          <FolderOpen size={15} />
        </IconButton>
      {/if}
      <IconButton variant="outline" onclick={onScan} title={$t("library.scan")} class="h-8 w-8 p-0">
        <ScanSearch size={15} />
      </IconButton>
    </div>
  </header>

  <div class="min-h-0 flex-1 overflow-y-auto">
    <div class="mx-auto max-w-4xl space-y-5 px-6 py-5">
      <section class="border-base-300 space-y-4 rounded-2xl border p-4">
        <div class="flex items-start justify-between gap-3">
          <p
            class="text-base-content-muted min-w-0 truncate text-xs"
            title={entry.path || undefined}
          >
            {entry.path || $t("projects.userHint")}
          </p>
          {#if entry.missing}
            <span class="tag tag-error inline-flex shrink-0 items-center gap-1">
              <AlertTriangle size={12} />
              {$t("scope.status.missing")}
            </span>
          {:else if entry.skills.length === 0}
            <span class="tag tag-neutral shrink-0">{$t("scope.status.empty")}</span>
          {:else if entry.driftCount > 0}
            <span class="tag tag-warning shrink-0">
              {$t("scope.status.changed", { count: entry.driftCount })}
            </span>
          {:else}
            <span class="tag tag-success inline-flex shrink-0 items-center gap-1">
              <CheckCircle2 size={12} />
              {$t("scope.status.synced")}
            </span>
          {/if}
        </div>

        <div class="space-y-2">
          <p class="text-base-content-subtle text-[11px] font-medium tracking-wide uppercase">
            {$t("scope.agents")}
          </p>
          <div class="flex flex-wrap items-center gap-3">
            {#each agentGroups as group (group.key)}
              <span class="group relative inline-flex">
                <AgentStack
                  agentIds={group.agentIds}
                  {agents}
                  size="md"
                  title={`${group.path}\n${$t("scope.agents.count", { count: group.skillCount })}`}
                />
                <button
                  class="border-base-300 bg-base-100 text-base-content-muted hover:border-error hover:text-error absolute -top-1.5 -right-1.5 hidden size-4 items-center justify-center rounded-full border group-hover:flex disabled:opacity-40"
                  type="button"
                  disabled={busy}
                  onclick={() => onRemoveAgents(group.agentIds)}
                  title={$t("scope.agents.remove")}
                  aria-label={$t("scope.agents.remove")}
                >
                  <X size={10} />
                </button>
              </span>
            {/each}
            <button
              class="border-base-300 text-base-content-muted hover:border-primary hover:text-primary flex size-8 items-center justify-center rounded-[0.65rem] border border-dashed transition disabled:opacity-40"
              type="button"
              onclick={onAddAgent}
              disabled={busy || !canAddAgent}
              title={entry.skills.length === 0
                ? $t("scope.agents.needSkill")
                : $t("scope.agents.add")}
              aria-label={$t("scope.agents.add")}
            >
              <Plus size={15} />
            </button>
          </div>
          {#if agentGroups.length === 0}
            <p class="text-base-content-faint text-xs">{$t("scope.agents.empty")}</p>
          {/if}
        </div>
      </section>

      {#if actionError}
        <p class="text-error text-sm whitespace-pre-wrap">{actionError}</p>
      {/if}

      <section class="space-y-2">
        <h3 class="text-base-content-subtle text-[11px] font-medium tracking-wide uppercase">
          {$t("scope.memory")}
        </h3>
        {#if presentMemory.length === 0}
          <p class="text-base-content-faint px-1 text-xs">{$t("scope.memory.empty")}</p>
        {:else}
          <div class="border-base-300 divide-base-300 divide-y rounded-2xl border">
            {#each presentMemory as file (file.path)}
              <div class="flex items-center gap-3 px-3 py-2">
                <button
                  class="flex min-w-0 flex-1 items-center gap-2.5 text-left"
                  type="button"
                  onclick={() => onOpenMemory(file)}
                  title={file.path}
                >
                  <span class="text-base-content-subtle shrink-0"><FileText size={15} /></span>
                  <span class="min-w-0 flex-1">
                    <span class="text-base-content block truncate text-[13px] font-medium">
                      {file.name}
                    </span>
                    <span class="text-base-content-faint block truncate text-[11px]">
                      {relativePath(file.path)}
                    </span>
                  </span>
                </button>
                <AgentStack agentIds={file.agentIds} {agents} title={file.path} />
              </div>
            {/each}
          </div>
        {/if}
      </section>

      <section class="space-y-2">
        <div class="flex items-center justify-between gap-3">
          <h3 class="text-base-content-subtle text-[11px] font-medium tracking-wide uppercase">
            {$t("scope.skills", { count: entry.skills.length })}
          </h3>
          <button
            class="border-base-300 text-base-content-muted hover:border-primary hover:text-primary flex h-7 items-center gap-1 rounded-lg border border-dashed px-2 text-[12px] transition disabled:opacity-50"
            type="button"
            onclick={onAddSkills}
            disabled={busy}
          >
            <Plus size={13} />
            <span>{$t("scope.addSkill")}</span>
          </button>
        </div>

        {#if entry.skills.length === 0}
          <p class="text-base-content-faint px-1 text-xs">{$t("scope.empty")}</p>
        {:else}
          <div class="border-base-300 divide-base-300 divide-y rounded-2xl border">
            {#each entry.skills as skill (skill.name)}
              {@const installs = installsOf(skill)}
              {@const drifted = installs.some((install) => install.state !== "in_sync")}
              <div class="flex items-center gap-3 px-3 py-2">
                <button
                  class="flex min-w-0 flex-1 items-center gap-2.5 text-left"
                  type="button"
                  onclick={() => onOpenSkill(skill.name)}
                  title={installs.map((install) => install.path).join("\n")}
                >
                  <span class="text-base-content-subtle shrink-0">
                    <SkillIcon source={skill.source} />
                  </span>
                  <span class="min-w-0 flex-1">
                    <span class="text-base-content block truncate text-[13px] font-medium">
                      {skill.name}
                    </span>
                    <span class="text-base-content-faint block truncate text-[11px]">
                      {locationLabel(installs)}
                    </span>
                  </span>
                </button>
                <div class="flex shrink-0 items-center gap-2">
                  {#if drifted}
                    <span class="tag tag-warning">{$t("library.tag.changed")}</span>
                  {/if}
                  <span class="flex items-center gap-2">
                    {#each installs as install (install.path)}
                      <AgentStack agentIds={install.agentIds} {agents} title={install.path} />
                    {/each}
                  </span>
                  <DropdownMenu
                    items={skillMenu(skill)}
                    label={$t("scope.skill.actions")}
                    disabled={busy}
                  />
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </section>
    </div>
  </div>
</div>
