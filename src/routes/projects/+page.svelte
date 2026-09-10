<script lang="ts">
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import { confirm } from "@tauri-apps/plugin-dialog";
  import ScopeList from "$lib/components/scopes/ScopeList.svelte";
  import ScopeDetail, { type ScopeSkillAction } from "$lib/components/scopes/ScopeDetail.svelte";
  import { t } from "$lib/i18n";
  import {
    buildInstructionsHref,
    buildLibraryHref,
    buildScopeHref,
    getAppLocation,
  } from "$lib/navigation/app-shell";
  import {
    openInFileManager,
    scopedInstalls,
    syncSkill,
    uninstallSkill,
    type HubSkillView,
  } from "$lib/api";
  import { homePath } from "$lib/stores/env";
  import { agents, agentsById, hubError, hubLoading, hubSkills, refreshHub } from "$lib/stores/hub";
  import { instructions } from "$lib/stores/instructions";
  import {
    openDiffModal,
    openImportModal,
    openInstallModal,
    openScopeAgentModal,
    openSkillPickerModal,
    openWorkspaceModal,
    performAction,
  } from "$lib/stores/modals";
  import {
    forgetProject,
    projectInstallCount,
    refreshUserProjects,
    refreshWorkspaces,
    userProjects,
    workspaces,
  } from "$lib/stores/user-projects";
  import { removeUserProject, removeWorkspace, type UserWorkspace } from "$lib/api/user-projects";
  import { listMemoryFiles, type MemoryFile } from "$lib/api/agent-apps";
  import { resolveAgents } from "$lib/agents";
  import { buildScopeEntries, parentDir, scopeKey, type ScopeEntry } from "$lib/scopes";

  let busy = $state(false);
  let actionError = $state("");
  let memoryFiles = $state<MemoryFile[]>([]);

  const location = $derived(getAppLocation(page.url));
  // The user scope is the home directory, shown like a project folder.
  const entries = $derived(
    buildScopeEntries($hubSkills, $userProjects, $t("scope.user"), $homePath)
  );
  const selectedKey = $derived(location.scope ? scopeKey(location.scope) : null);
  // Template name per instruction file that is installed from one.
  const libraryNames = $derived.by(() => {
    const map: Record<string, string> = {};
    for (const item of $instructions) {
      for (const install of item.installs) map[install.path] = item.name;
    }
    return map;
  });

  const selected = $derived<ScopeEntry | null>(
    entries.find((entry) => entry.key === selectedKey) ?? null
  );
  const availableAgents = $derived.by(() => {
    const entry = selected;
    if (!entry) return [];
    const installed = new Set(
      entry.skills.flatMap((skill) =>
        scopedInstalls(skill, entry.ref).flatMap((install) => install.agentIds)
      )
    );
    return $agents.filter((agent) => {
      if (installed.has(agent.id)) return false;
      return entry.ref.scope === "project"
        ? Boolean(agent.project_path)
        : Boolean(agent.global_path);
    });
  });

  // The instruction files of the selected scope, refreshed whenever it changes.
  $effect(() => {
    const ref = selected?.ref;
    if (!ref) {
      memoryFiles = [];
      return;
    }
    let current = true;
    listMemoryFiles(ref.scope, ref.projectPath)
      .then((files) => {
        if (current) memoryFiles = files;
      })
      .catch((error) => console.error(error));
    return () => {
      current = false;
    };
  });

  async function runAction(work: () => Promise<unknown>) {
    if (busy) return;
    busy = true;
    actionError = "";
    try {
      await work();
    } catch (error) {
      actionError = String(error);
    } finally {
      busy = false;
      await refreshHub().catch(console.error);
    }
  }

  function handleRemoveWorkspace(workspace: UserWorkspace) {
    void runAction(async () => {
      const confirmed = await confirm($t("workspace.removeConfirm", { name: workspace.name }), {
        title: $t("workspace.remove"),
        kind: "warning",
      });
      if (!confirmed) return;
      await removeWorkspace(workspace.path, false);
      await refreshWorkspaces();
      await refreshUserProjects();
    });
  }

  function handleRemoveProject(entry: ScopeEntry) {
    void runAction(async () => {
      const confirmed = await confirm($t("scope.project.removeConfirm", { name: entry.name }), {
        title: $t("scope.project.remove"),
        kind: "warning",
      });
      if (!confirmed) return;
      await removeUserProject(entry.name);
      await refreshUserProjects();
    });
  }

  /** The folder is gone: drop the project and every record that points into it. */
  function handleForgetProject(entry: ScopeEntry) {
    void runAction(async () => {
      const confirmed = await confirm(
        $t("scope.project.forgetConfirm", {
          name: entry.name,
          path: entry.path,
          count: projectInstallCount(entry.path),
        }),
        { title: $t("scope.project.forget"), kind: "warning" }
      );
      if (!confirmed) return;
      const failures = await forgetProject(entry.path, entry.unregistered ? null : entry.name);
      if (failures.length > 0) actionError = failures.join("\n");
      if (selectedKey === entry.key) {
        await goto(buildScopeHref(), { replaceState: true, keepFocus: true, noScroll: true });
      }
    });
  }

  function handleSelect(entry: ScopeEntry) {
    actionError = "";
    goto(buildScopeHref(entry.ref), { replaceState: true, keepFocus: true, noScroll: true });
  }

  function handleOpenDir() {
    const entry = selected;
    if (!entry?.path) return;
    openInFileManager(entry.path).catch((error) => (actionError = String(error)));
  }

  function handleAddAgent() {
    const entry = selected;
    if (!entry) return;
    const existingIds = [
      ...new Set(
        entry.skills.flatMap((skill) =>
          scopedInstalls(skill, entry.ref).flatMap((install) => install.agentIds)
        )
      ),
    ];
    openScopeAgentModal({
      scope: entry.ref,
      scopeName: entry.name,
      skillNames: entry.skills.map((skill) => skill.name),
      existingIds,
    });
  }

  /**
   * Remove agents that share a directory: they are installed together, so every skill of
   * this scope is uninstalled from all of them at once.
   */
  function handleRemoveAgents(location: { path: string; agentIds: string[] }) {
    const entry = selected;
    if (!entry) return;
    // The tile is a directory; an agent id would also match every other directory it reads.
    const affected = entry.skills
      .map((skill) => ({
        skill,
        paths: scopedInstalls(skill, entry.ref)
          .filter((install) => parentDir(install.path) === location.path)
          .map((install) => install.path),
      }))
      .filter((item) => item.paths.length > 0);
    if (affected.length === 0) return;
    const label = resolveAgents(location.agentIds, $agentsById)[0]?.name ?? location.path;
    void runAction(async () => {
      const confirmed = await confirm(
        $t("scope.agents.removeConfirm", { agent: label, count: affected.length }),
        { title: $t("scope.agents.remove"), kind: "warning" }
      );
      if (!confirmed) return;
      const failures: string[] = [];
      for (const { skill, paths } of affected) {
        const result = await performAction((force) =>
          uninstallSkill({ name: skill.name, targets: [], paths, force })
        );
        if (!result.applied) failures.push(`${skill.name}: ${result.blockers.join("; ")}`);
      }
      if (failures.length > 0) actionError = failures.join("\n");
    });
  }

  function handleSkillAction(skill: HubSkillView, action: ScopeSkillAction) {
    const entry = selected;
    if (!entry) return;
    const installs = scopedInstalls(skill, entry.ref);
    const drifted = installs.filter((install) => install.state !== "in_sync");
    const name = skill.name;

    switch (action) {
      case "manage":
        openInstallModal([name], {
          scope: entry.ref.scope,
          projectPath: entry.ref.projectPath,
          lockScope: true,
        });
        return;
      case "diff": {
        const target = drifted.find((install) => install.mode === "copy");
        if (target) openDiffModal(name, { kind: "target", path: target.path });
        return;
      }
      case "openDir": {
        const target = installs[0];
        if (target) openInFileManager(target.path).catch((error) => (actionError = String(error)));
        return;
      }
      case "push":
        void runAction(async () => {
          const targets = drifted.map((install) => install.path);
          if (targets.length === 0) return;
          const result = await performAction((force) =>
            syncSkill(name, { kind: "push_targets", targets, force })
          );
          if (!result.applied && result.blockers.length > 0) {
            actionError = result.blockers.join("\n");
          }
        });
        return;
      case "adopt":
        void runAction(async () => {
          const target = drifted.find(
            (install) => install.state === "modified" || install.state === "conflict"
          );
          if (!target) return;
          const result = await performAction((force) =>
            syncSkill(name, { kind: "adopt_target", path: target.path, force })
          );
          if (!result.applied && result.blockers.length > 0) {
            actionError = result.blockers.join("\n");
          }
        });
        return;
      case "uninstall":
        void runAction(async () => {
          const targets = installs.flatMap((install) =>
            install.agentIds.map((agentId) => ({
              scope: entry.ref.scope,
              projectPath: entry.ref.projectPath,
              agentId,
            }))
          );
          if (targets.length === 0) return;
          const result = await performAction((force) => uninstallSkill({ name, targets, force }));
          if (!result.applied && result.blockers.length > 0) {
            actionError = result.blockers.join("\n");
          }
        });
        return;
    }
  }
</script>

<section class="flex h-full min-h-0 min-w-0 flex-col overflow-hidden">
  <div
    class="grid min-h-0 flex-1 grid-cols-[18rem_minmax(0,1fr)] max-[1000px]:grid-cols-[15rem_minmax(0,1fr)]"
  >
    <ScopeList
      {entries}
      workspaces={$workspaces}
      {selectedKey}
      loading={$hubLoading}
      error={$hubError}
      onSelect={handleSelect}
      onAddWorkspace={() => openWorkspaceModal({ mode: "add" })}
      onRescanWorkspace={(workspace) =>
        openWorkspaceModal({ mode: "rescan", name: workspace.name, path: workspace.path })}
      onRemoveWorkspace={handleRemoveWorkspace}
      onRemoveProject={handleRemoveProject}
      onForgetProject={handleForgetProject}
      onRefresh={() => refreshHub().catch(console.error)}
    />

    {#if selected}
      {#key selected.key}
        <ScopeDetail
          entry={selected}
          agents={$agentsById}
          {availableAgents}
          {memoryFiles}
          {libraryNames}
          {busy}
          {actionError}
          onOpenDir={handleOpenDir}
          onForgetProject={() => handleForgetProject(selected)}
          onScan={() =>
            openImportModal({
              tab: "folder",
              folder: selected.kind === "project" ? selected.path : null,
            })}
          onAddSkills={() => openSkillPickerModal(selected.ref)}
          onAddAgent={handleAddAgent}
          onRemoveAgents={handleRemoveAgents}
          onOpenMemory={(file) => goto(buildInstructionsHref({ file: file.path }))}
          onOpenSkill={(name) => goto(buildLibraryHref({ skill: name }))}
          onSkillAction={handleSkillAction}
        />
      {/key}
    {:else}
      <div class="flex min-h-0 flex-col">
        <header class="border-base-300 h-12 flex-none border-b" data-window-drag-region></header>
        <div
          class="text-base-content-muted flex flex-1 items-center justify-center p-8 text-center text-sm"
        >
          {$t("projects.selectHint")}
        </div>
      </div>
    {/if}
  </div>
</section>
