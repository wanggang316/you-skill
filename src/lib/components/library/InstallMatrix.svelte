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

  type Row = {
    key: string;
    label: string;
    scope: InstallScope;
    projectPath: string | null;
    subtitle: string | null;
    missing: boolean;
    unregistered: boolean;
    installs: InstallView[];
  };

  let openChip = $state<string | null>(null);

  const baseName = (path: string) => path.split(/[/\\]/).filter(Boolean).pop() || path;

  const rows = $derived.by((): Row[] => {
    const userInstalls = skill.installs.filter((install) => install.scope === "user");
    const result: Row[] = [
      {
        key: "user",
        label: $t("detail.installs.user"),
        scope: "user",
        projectPath: null,
        subtitle: null,
        missing: false,
        unregistered: false,
        installs: userInstalls,
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

  const chipKey = (install: InstallView, agentId: string) => `${install.path}|${agentId}`;

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

  function toggleChip(key: string) {
    openChip = openChip === key ? null : key;
  }

  function handleAction(install: InstallView, action: TargetAction, agentId: string) {
    openChip = null;
    onTargetAction(install, action, agentId);
  }

  $effect(() => {
    if (!openChip) return;
    const close = (event: MouseEvent) => {
      const target = event.target as HTMLElement | null;
      if (target?.closest("[data-target-chip]")) return;
      openChip = null;
    };
    document.addEventListener("mousedown", close);
    return () => document.removeEventListener("mousedown", close);
  });
</script>

<div class="border-base-300 divide-base-300 divide-y rounded-2xl border">
  {#each rows as row (row.key)}
    <div class="flex items-start gap-3 px-4 py-3">
      <div class="w-40 min-w-0 shrink-0">
        <p class="text-base-content flex items-center gap-1.5 truncate text-sm font-medium">
          <span class="truncate">{row.label}</span>
          {#if row.missing}
            <span class="text-error" title={$t("detail.installs.projectMissing")}>
              <AlertTriangle size={13} />
            </span>
          {/if}
        </p>
        {#if row.subtitle}
          <p class="text-base-content-faint truncate text-[11px]" title={row.subtitle}>
            {row.subtitle}
          </p>
        {/if}
        {#if row.unregistered}
          <span class="tag tag-neutral mt-1 inline-block">{$t("detail.installs.unregistered")}</span
          >
        {/if}
      </div>

      <div class="flex min-w-0 flex-1 flex-wrap items-center gap-2">
        {#each row.installs as install (install.path)}
          {#each install.agentIds as agentId (agentId)}
            {@const key = chipKey(install, agentId)}
            <div class="relative" data-target-chip>
              <button
                class={`border-base-300 hover:bg-base-200 relative flex items-center gap-1.5 rounded-xl border py-1 pr-2 pl-1 text-[12px] transition ${
                  openChip === key ? "bg-base-200" : "bg-base-100"
                }`}
                type="button"
                title={`${agentName(agentId)} · ${$t(`target.state.${install.state}`)} · ${$t(`target.mode.${install.mode}`)}`}
                onclick={() => toggleChip(key)}
                disabled={busy}
              >
                <AgentAppIcon {agentId} name={agentName(agentId)} />
                <span class="text-base-content max-w-24 truncate">{agentName(agentId)}</span>
                {#if install.mode === "symlink"}
                  <Link2 size={12} class="text-base-content-subtle" />
                {/if}
                <span class={`state-dot state-${install.state}`}></span>
              </button>

              {#if openChip === key}
                <div
                  class="border-base-300 bg-base-100 absolute top-full left-0 z-30 mt-1 w-72 rounded-xl border p-3 text-xs shadow-xl"
                >
                  <div class="flex items-center justify-between gap-2">
                    <span class="text-base-content font-medium">{agentName(agentId)}</span>
                    <span class="flex items-center gap-1.5">
                      <span class={`state-dot state-${install.state}`}></span>
                      <span class="text-base-content-muted">
                        {$t(`target.state.${install.state}`)}
                      </span>
                    </span>
                  </div>
                  <dl class="text-base-content-muted mt-2 space-y-1">
                    <div class="flex gap-2">
                      <dt class="w-12 shrink-0">{$t("target.mode")}</dt>
                      <dd>{$t(`target.mode.${install.mode}`)}</dd>
                    </div>
                    <div class="flex gap-2">
                      <dt class="w-12 shrink-0">{$t("target.path")}</dt>
                      <dd class="break-all" title={install.path}>{install.path}</dd>
                    </div>
                    {#if install.agentIds.length > 1}
                      <div class="flex gap-2">
                        <dt class="w-12 shrink-0">{$t("target.agents")}</dt>
                        <dd>{install.agentIds.map(agentName).join(", ")}</dd>
                      </div>
                    {/if}
                  </dl>
                  <div class="mt-3 flex flex-wrap gap-1.5">
                    {#each actionsFor(install) as action}
                      <button
                        class={`rounded-lg border px-2 py-1 transition ${
                          action === "uninstall"
                            ? "border-error-border text-error hover:bg-error/10"
                            : "border-base-300 text-base-content hover:bg-base-200"
                        }`}
                        type="button"
                        onclick={() => handleAction(install, action, agentId)}
                      >
                        {actionLabel(action)}
                      </button>
                    {/each}
                  </div>
                </div>
              {/if}
            </div>
          {/each}
        {/each}
        <button
          class="border-base-300 text-base-content-muted hover:border-primary hover:text-primary flex h-8 items-center gap-1 rounded-xl border border-dashed px-2 text-[12px] transition"
          type="button"
          onclick={() => onAdd(row.scope, row.projectPath)}
          disabled={busy || row.missing}
          title={$t("detail.installs.add")}
        >
          <Plus size={13} />
          {#if row.installs.length === 0}
            <span>{$t("detail.installs.add")}</span>
          {/if}
        </button>
      </div>
    </div>
  {/each}
</div>
