<script lang="ts">
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import { confirm } from "@tauri-apps/plugin-dialog";
  import ScopeList from "$lib/components/scopes/ScopeList.svelte";
  import ScopeDetail, { type ScopeSkillAction } from "$lib/components/scopes/ScopeDetail.svelte";
  import { t } from "$lib/i18n";
  import { buildLibraryHref, buildScopeHref, getAppLocation } from "$lib/navigation/app-shell";
  import {
    openInFileManager,
    scopedInstalls,
    syncSkill,
    uninstallSkill,
    type HubSkillView,
  } from "$lib/api";
  import { agents, agentsById, hubError, hubLoading, hubSkills, refreshHub } from "$lib/stores/hub";
  import {
    openDiffModal,
    openImportModal,
    openInstallModal,
    openProjectFormModal,
    openScopeAgentModal,
    openSkillPickerModal,
    openWorkspaceModal,
    performAction,
  } from "$lib/stores/modals";
  import {
    refreshUserProjects,
    refreshWorkspaces,
    userProjects,
    workspaces,
  } from "$lib/stores/user-projects";
  import { removeWorkspace, type UserWorkspace } from "$lib/api/user-projects";
  import { listMemoryFiles, type MemoryFile } from "$lib/api/agent-apps";
  import { resolveAgents } from "$lib/agents";
  import { buildScopeEntries, scopeKey, type ScopeEntry } from "$lib/scopes";

  let busy = $state(false);
  let actionError = $state("");
  let memoryFiles = $state<MemoryFile[]>([]);

  const location = $derived(getAppLocation(page.url));
  const entries = $derived(buildScopeEntries($hubSkills, $userProjects, $t("scope.user")));
  const selectedKey = $derived(location.scope ? scopeKey(location.scope) : null);
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
  function handleRemoveAgents(agentIds: string[]) {
    const entry = selected;
    if (!entry || agentIds.length === 0) return;
    const affected = entry.skills.filter((skill) =>
      scopedInstalls(skill, entry.ref).some((install) =>
        install.agentIds.some((id) => agentIds.includes(id))
      )
    );
    if (affected.length === 0) return;
    const label = resolveAgents(agentIds, $agentsById)[0]?.name ?? agentIds[0];
    void runAction(async () => {
      const confirmed = await confirm(
        $t("scope.agents.removeConfirm", { agent: label, count: affected.length }),
        { title: $t("scope.agents.remove"), kind: "warning" }
      );
      if (!confirmed) return;
      const failures: string[] = [];
      for (const skill of affected) {
        const result = await performAction((force) =>
          uninstallSkill({
            name: skill.name,
            targets: agentIds.map((agentId) => ({
              scope: entry.ref.scope,
              projectPath: entry.ref.projectPath,
              agentId,
            })),
            force,
          })
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
        openInstallModal([name], { scope: entry.ref.scope, projectPath: entry.ref.projectPath });
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
      onManageProjects={() => openProjectFormModal()}
      onRefresh={() => refreshHub().catch(console.error)}
    />

    {#if selected}
      {#key selected.key}
        <ScopeDetail
          entry={selected}
          agents={$agentsById}
          {availableAgents}
          {memoryFiles}
          {busy}
          {actionError}
          onOpenDir={handleOpenDir}
          onScan={() => openImportModal({ tab: "folder", folder: selected.path || null })}
          onAddSkills={() => openSkillPickerModal(selected.ref)}
          onAddAgent={handleAddAgent}
          onRemoveAgents={handleRemoveAgents}
          onOpenMemory={(file) =>
            openInFileManager(file.path).catch((error) => (actionError = String(error)))}
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
