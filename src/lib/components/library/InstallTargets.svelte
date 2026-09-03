<script lang="ts">
  import { AlertTriangle, Link2, Plus } from "@lucide/svelte";
  import AgentAppIcon from "$lib/components/AgentAppIcon.svelte";
  import { t } from "$lib/i18n";
  import type { HubSkillView, InstallScope, InstallView } from "$lib/api/hub";
  import type { AgentInfo } from "$lib/api/skills";
  import type { UserProject } from "$lib/api/user-projects";

  export type TargetAction = "push" | "adopt" | "uninstall" | "reinstall";

  let {
    skill,
    projects = [],
    agents,
    busy = false,
    onAdd,
    onTargetAction,
  }: {
    skill: HubSkillView;
    projects?: UserProject[];
    agents: Map<string, AgentInfo>;
    busy?: boolean;
    onAdd: (scope: InstallScope, projectPath: string | null) => void;
    onTargetAction: (install: InstallView, action: TargetAction, agentId: string) => void;
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

  function actionsFor(install: InstallView): TargetAction[] {
    const isLink = install.mode === "symlink";
    switch (install.state) {
      case "outdated":
        return isLink ? ["uninstall"] : ["push", "uninstall"];
      case "modified":
      case "conflict":
        return isLink ? ["uninstall"] : ["adopt", "push", "uninstall"];
      case "missing":
      case "broken_link":
        return ["reinstall", "uninstall"];
      default:
        return ["uninstall"];
    }
  }

  function actionLabel(action: TargetAction): string {
    switch (action) {
      case "push":
        return $t("target.push");
      case "adopt":
        return $t("target.adopt");
      case "reinstall":
        return $t("target.reinstall");
      default:
        return $t("target.uninstall");
    }
  }
</script>

<div class="border-base-300 divide-base-300 divide-y rounded-2xl border">
  {#each groups as group (group.key)}
    <div>
      <div class="flex items-center justify-between gap-3 px-4 py-2.5">
        <div class="min-w-0">
          <p class="text-base-content flex items-center gap-1.5 text-sm font-medium">
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
          {#if group.subtitle}
            <p class="text-base-content-faint truncate text-[11px]" title={group.subtitle}>
              {group.subtitle}
            </p>
          {/if}
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

      {#each group.installs as install (install.path)}
        {#each install.agentIds as agentId (agentId)}
          <div
            class="border-base-300 flex items-center gap-3 border-t border-dashed py-2 pr-4 pl-5"
          >
            <AgentAppIcon {agentId} name={agentName(agentId)} />
            <div class="flex min-w-0 flex-1 flex-col">
              <div class="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-0.5 text-[13px]">
                <span class="text-base-content truncate font-medium">{agentName(agentId)}</span>
                <span class="text-base-content-subtle inline-flex items-center gap-1 text-[11px]">
                  {#if install.mode === "symlink"}
                    <Link2 size={11} />
                  {/if}
                  {$t(`target.mode.${install.mode}`)}
                </span>
                <span class="inline-flex items-center gap-1 text-[11px]">
                  <span class={`state-dot state-${install.state}`}></span>
                  <span class="text-base-content-muted">
                    {$t(`target.state.${install.state}`)}
                  </span>
                </span>
              </div>
              <p class="text-base-content-faint truncate text-[11px]" title={install.path}>
                {install.path}
              </p>
            </div>
            <div class="flex shrink-0 flex-wrap justify-end gap-1.5">
              {#each actionsFor(install) as action}
                <button
                  class={`rounded-lg border px-2 py-1 text-[11px] transition disabled:opacity-50 ${
                    action === "uninstall"
                      ? "border-error-border text-error hover:bg-error/10"
                      : "border-base-300 text-base-content hover:bg-base-200"
                  }`}
                  type="button"
                  disabled={busy}
                  onclick={() => onTargetAction(install, action, agentId)}
                >
                  {actionLabel(action)}
                </button>
              {/each}
            </div>
          </div>
        {/each}
      {/each}
    </div>
  {/each}
</div>
