<script lang="ts">
  import { AlertTriangle, Plus } from "@lucide/svelte";
  import AgentAppIcon from "$lib/components/AgentAppIcon.svelte";
  import { t } from "$lib/i18n";
  import type { HubSkillView, InstallScope, InstallView } from "$lib/api/hub";
  import type { AgentInfo } from "$lib/api/skills";
  import type { UserProject } from "$lib/api/user-projects";

  let {
    skill,
    projects = [],
    agents,
    busy = false,
    onAdd,
  }: {
    skill: HubSkillView;
    projects?: UserProject[];
    agents: Map<string, AgentInfo>;
    busy?: boolean;
    onAdd: (scope: InstallScope, projectPath: string | null) => void;
  } = $props();

  type Group = {
    key: string;
    label: string;
    scope: InstallScope;
    projectPath: string | null;
    subtitle: string | null;
    missing: boolean;
    unregistered: boolean;
    installs: InstallView[];
  };

  const baseName = (path: string) => path.split(/[/\\]/).filter(Boolean).pop() || path;

  const groups = $derived.by((): Group[] => {
    const result: Group[] = [
      {
        key: "user",
        label: $t("detail.installs.user"),
        scope: "user",
        projectPath: null,
        subtitle: null,
        missing: false,
        unregistered: false,
        installs: skill.installs.filter((install) => install.scope === "user"),
      },
    ];
    const seen = new Set<string>();
    for (const project of projects) {
      seen.add(project.path);
      const installs = skill.installs.filter(
        (install) => install.scope === "project" && install.projectPath === project.path
      );
      result.push({
        key: `project:${project.path}`,
        label: project.name,
        scope: "project",
        projectPath: project.path,
        subtitle: project.path,
        missing: installs.some((install) => install.projectMissing),
        unregistered: false,
        installs,
      });
    }
    for (const install of skill.installs) {
      if (install.scope !== "project" || !install.projectPath || seen.has(install.projectPath)) {
        continue;
      }
      seen.add(install.projectPath);
      result.push({
        key: `project:${install.projectPath}`,
        label: baseName(install.projectPath),
        scope: "project",
        projectPath: install.projectPath,
        subtitle: install.projectPath,
        missing: install.projectMissing,
        unregistered: true,
        installs: skill.installs.filter(
          (item) => item.scope === "project" && item.projectPath === install.projectPath
        ),
      });
    }
    return result;
  });

  function agentName(id: string): string {
    return agents.get(id)?.display_name ?? id;
  }

  function chipTitle(install: InstallView, agentId: string): string {
    const detail = `${$t(`target.mode.${install.mode}`)} · ${$t(`target.state.${install.state}`)}`;
    return `${agentName(agentId)}\n${install.path}\n${detail}`;
  }
</script>

<div class="border-base-300 divide-base-300 divide-y rounded-2xl border">
  {#each groups as group (group.key)}
    <div class="px-4 py-2.5">
      <div class="flex items-center justify-between gap-3">
        <div class="min-w-0">
          <p
            class="text-base-content flex items-center gap-1.5 text-sm font-medium"
            title={group.subtitle ?? undefined}
          >
            <span class="truncate">{group.label}</span>
            {#if group.missing}
              <span class="text-error shrink-0" title={$t("detail.installs.projectMissing")}>
                <AlertTriangle size={13} />
              </span>
            {/if}
            {#if group.unregistered}
              <span class="tag tag-neutral shrink-0">{$t("detail.installs.unregistered")}</span>
            {/if}
          </p>
        </div>
        <button
          class="border-base-300 text-base-content-muted hover:border-primary hover:text-primary flex h-7 shrink-0 items-center gap-1 rounded-lg border border-dashed px-2 text-[12px] transition disabled:opacity-50"
          type="button"
          onclick={() => onAdd(group.scope, group.projectPath)}
          disabled={busy || group.missing}
          title={$t("detail.installs.add")}
        >
          <Plus size={13} />
          <span>{$t("detail.installs.add")}</span>
        </button>
      </div>

      {#if group.installs.length > 0}
        <div class="mt-2 flex flex-wrap items-center gap-1.5">
          {#each group.installs as install (install.path)}
            {#each install.agentIds as agentId (agentId)}
              <span title={chipTitle(install, agentId)}>
                <AgentAppIcon {agentId} name={agentName(agentId)} size="sm" />
              </span>
            {/each}
          {/each}
        </div>
      {/if}
    </div>
  {/each}
</div>
