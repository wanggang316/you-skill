<script lang="ts">
  import { AlertTriangle, CheckCircle2, FolderOpen, Plus, ScanSearch, X } from "@lucide/svelte";
  import AgentAppIcon from "$lib/components/AgentAppIcon.svelte";
  import SkillIcon from "$lib/components/SkillIcon.svelte";
  import DropdownMenu, { type MenuItem } from "$lib/components/ui/DropdownMenu.svelte";
  import IconButton from "$lib/components/ui/IconButton.svelte";
  import { t } from "$lib/i18n";
  import { scopedInstalls, type HubSkillView, type InstallView } from "$lib/api/hub";
  import type { AgentInfo } from "$lib/api/skills";
  import type { ScopeEntry } from "$lib/scopes";

  export type ScopeSkillAction = "manage" | "diff" | "push" | "adopt" | "openDir" | "uninstall";

  let {
    entry,
    agents,
    availableAgents = [],
    busy = false,
    actionError = "",
    onOpenDir,
    onScan,
    onAddSkills,
    onAddAgent,
    onRemoveAgent,
    onOpenSkill,
    onSkillAction,
  }: {
    entry: ScopeEntry;
    agents: Map<string, AgentInfo>;
    /** Agents that can still be added to this scope. */
    availableAgents?: AgentInfo[];
    busy?: boolean;
    actionError?: string;
    onOpenDir: () => void;
    onScan: () => void;
    onAddSkills: () => void;
    onAddAgent: () => void;
    onRemoveAgent: (agentId: string) => void;
    onOpenSkill: (name: string) => void;
    onSkillAction: (skill: HubSkillView, action: ScopeSkillAction) => void;
  } = $props();

  type ScopeAgent = { id: string; name: string; skillCount: number };

  const scopeAgents = $derived.by((): ScopeAgent[] => {
    const counts = new Map<string, number>();
    for (const skill of entry.skills) {
      const ids = new Set(scopedInstalls(skill, entry.ref).flatMap((install) => install.agentIds));
      for (const id of ids) counts.set(id, (counts.get(id) ?? 0) + 1);
    }
    return [...counts.entries()].map(([id, skillCount]) => ({
      id,
      name: agents.get(id)?.display_name ?? id,
      skillCount,
    }));
  });

  const canAddAgent = $derived(availableAgents.length > 0 && entry.skills.length > 0);

  function installsOf(skill: HubSkillView): InstallView[] {
    return scopedInstalls(skill, entry.ref);
  }

  function locationLabel(installs: InstallView[]): string {
    if (installs.length !== 1) return $t("scope.skill.locations", { count: installs.length });
    const path = installs[0].path;
    if (entry.path && path.startsWith(entry.path)) {
      return path.slice(entry.path.length).replace(/^[/\\]+/, "");
    }
    return path;
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
          <div class="flex flex-wrap items-center gap-2">
            {#each scopeAgents as agent (agent.id)}
              <span
                class="group relative inline-flex"
                title={`${agent.name} · ${$t("scope.agents.count", { count: agent.skillCount })}`}
              >
                <AgentAppIcon agentId={agent.id} name={agent.name} />
                <span
                  class="text-success bg-base-100 absolute -right-1 -bottom-1 rounded-full leading-none"
                >
                  <CheckCircle2 size={12} />
                </span>
                <button
                  class="border-base-300 bg-base-100 text-base-content-muted hover:border-error hover:text-error absolute -top-1.5 -right-1.5 hidden size-4 items-center justify-center rounded-full border group-hover:flex disabled:opacity-40"
                  type="button"
                  disabled={busy}
                  onclick={() => onRemoveAgent(agent.id)}
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
          {#if scopeAgents.length === 0}
            <p class="text-base-content-faint text-xs">{$t("scope.agents.empty")}</p>
          {/if}
        </div>
      </section>

      {#if actionError}
        <p class="text-error text-sm whitespace-pre-wrap">{actionError}</p>
      {/if}

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
                  <span class="flex items-center gap-1">
                    {#each installs as install (install.path)}
                      {#each install.agentIds as agentId (agentId)}
                        <span title={agents.get(agentId)?.display_name ?? agentId}>
                          <AgentAppIcon
                            {agentId}
                            name={agents.get(agentId)?.display_name ?? agentId}
                            size="sm"
                          />
                        </span>
                      {/each}
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
