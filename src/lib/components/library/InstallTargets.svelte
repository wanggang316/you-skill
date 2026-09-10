<script lang="ts">
  import { AlertTriangle, Folder, Plus, UserRound } from "@lucide/svelte";
  import AgentBadge from "$lib/components/AgentBadge.svelte";
  import { t } from "$lib/i18n";
  import { baseName } from "$lib/scopes";
  import type { InstallScope, InstallView } from "$lib/api/hub";
  import type { AgentInfo } from "$lib/api/skills";
  import type { UserProject } from "$lib/api/user-projects";

  let {
    installs = [],
    projects = [],
    agents,
    homePath = "",
    busy = false,
    onAdd,
  }: {
    /** Install records of one skill or instruction. */
    installs?: InstallView[];
    projects?: UserProject[];
    agents: Map<string, AgentInfo>;
    /** Home directory: the user row is named after it, like a project after its folder. */
    homePath?: string;
    busy?: boolean;
    /** `lockScope` is set: the row decides the target, the dialog must not offer another. */
    onAdd: (scope: InstallScope, projectPath: string | null, lockScope: boolean) => void;
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

  const userGroup = $derived<Group>({
    key: "user",
    label: baseName(homePath) || $t("detail.installs.user"),
    scope: "user",
    projectPath: null,
    subtitle: homePath || null,
    missing: false,
    unregistered: false,
    installs: installs.filter((install) => install.scope === "user"),
  });

  /** Only projects that have the skill; registered ones first, under their own name. */
  const projectGroups = $derived.by((): Group[] => {
    const result: Group[] = [];
    const seen = new Set<string>();
    const installsOf = (path: string) =>
      installs.filter((install) => install.scope === "project" && install.projectPath === path);
    for (const project of projects) {
      const installs = installsOf(project.path);
      if (installs.length === 0) continue;
      seen.add(project.path);
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
    for (const install of installs) {
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
        installs: installsOf(install.projectPath),
      });
    }
    return result;
  });

  /** Only locations that hold the entry; empty ones are added from the header. */
  const groups = $derived(
    userGroup.installs.length > 0 ? [userGroup, ...projectGroups] : projectGroups
  );

  const stateNote = (install: InstallView) =>
    `${$t(`target.mode.${install.mode}`)} · ${$t(`target.state.${install.state}`)}`;
</script>

{#snippet groupRow(group: Group)}
  <div class="px-4 py-2.5">
    <div class="flex items-center justify-between gap-3">
      <p
        class="text-base-content flex min-w-0 items-center gap-2 text-sm font-medium"
        title={group.subtitle ?? undefined}
      >
        <span class="text-base-content-subtle shrink-0">
          {#if group.scope === "user"}
            <UserRound size={14} />
          {:else}
            <Folder size={14} />
          {/if}
        </span>
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
    <div class="mt-2 flex flex-wrap items-center gap-3">
      {#each group.installs as install (install.path)}
        <AgentBadge
          agentIds={install.agentIds}
          {agents}
          path={install.path}
          note={stateNote(install)}
        />
      {/each}
      <button
        class="border-base-300 text-base-content-muted hover:border-primary hover:text-primary flex size-6 items-center justify-center rounded-lg border border-dashed transition disabled:opacity-40"
        type="button"
        onclick={() => onAdd(group.scope, group.projectPath, true)}
        disabled={busy || group.missing}
        title={$t("detail.installs.add")}
        aria-label={$t("detail.installs.add")}
      >
        <Plus size={13} />
      </button>
    </div>
  </div>
{/snippet}

{#if groups.length > 0}
  <div class="border-base-300 divide-base-300 divide-y rounded-2xl border">
    {#each groups as group (group.key)}
      {@render groupRow(group)}
    {/each}
  </div>
{/if}
