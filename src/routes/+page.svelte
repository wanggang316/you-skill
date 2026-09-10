<script lang="ts">
  import { Info, X } from "@lucide/svelte";
  import { confirm } from "@tauri-apps/plugin-dialog";
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import SkillList from "$lib/components/library/SkillList.svelte";
  import SkillDetail from "$lib/components/library/SkillDetail.svelte";
  import type { TargetAction } from "$lib/components/library/DriftPanel.svelte";
  import RemoveSkillModal from "$lib/components/RemoveSkillModal.svelte";
  import { t } from "$lib/i18n";
  import { buildLibraryHref, getAppLocation, type LibraryFilter } from "$lib/navigation/app-shell";
  import {
    installSkill,
    openInFileManager,
    removeHubSkill,
    syncSkill,
    uninstallSkill,
    type HubSkillView,
    type InstallScope,
    type InstallView,
    type SyncAction,
  } from "$lib/api";
  import {
    agentsById,
    checkSourceUpdates,
    hubError,
    hubLoading,
    hubSkills,
    migrationReport,
    refreshHub,
    sourceChecking,
  } from "$lib/stores/hub";
  import { homePath } from "$lib/stores/env";
  import {
    openImportModal,
    openInstallModal,
    openLocationPickerModal,
    performAction,
  } from "$lib/stores/modals";
  import { userProjects } from "$lib/stores/user-projects";
  import { baseName } from "$lib/scopes";

  let search = $state("");
  let busy = $state(false);
  let actionError = $state("");
  let removeModalOpen = $state(false);
  let migrationDismissed = $state(false);

  const location = $derived(getAppLocation(page.url));
  const selectedName = $derived(location.skill);
  const filter = $derived(location.filter);

  const filteredSkills = $derived.by(() => {
    const needle = search.trim().toLowerCase();
    return $hubSkills.filter((skill) => {
      if (needle) {
        const haystack = `${skill.name} ${skill.description ?? ""}`.toLowerCase();
        if (!haystack.includes(needle)) return false;
      }
      switch (filter) {
        case "changed":
          return skill.hasDrift;
        case "uninstalled":
          return skill.installs.length === 0;
        default:
          return true;
      }
    });
  });

  const selectedSkill = $derived<HubSkillView | null>(
    selectedName ? ($hubSkills.find((skill) => skill.name === selectedName) ?? null) : null
  );

  const migrationNotice = $derived.by(() => {
    const report = $migrationReport;
    if (!report || migrationDismissed) return null;
    if (report.errors.length > 0) {
      return { kind: "errors" as const, count: report.errors.length, details: report.errors };
    }
    if (report.imported.length > 0 && isRecent(report.completedAt)) {
      return { kind: "imported" as const, count: report.imported.length, details: [] };
    }
    return null;
  });

  function isRecent(value: string | null | undefined): boolean {
    if (!value) return false;
    const time = new Date(value).getTime();
    return Number.isFinite(time) && Date.now() - time < 24 * 60 * 60 * 1000;
  }

  function navigate(options: { skill?: string | null; filter?: LibraryFilter }) {
    const href = buildLibraryHref({
      skill: options.skill === undefined ? selectedName : options.skill,
      filter: options.filter ?? filter,
    });
    goto(href, { replaceState: true, keepFocus: true, noScroll: true });
  }

  function handleSelect(name: string) {
    actionError = "";
    navigate({ skill: name });
  }

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

  function handleInstall(scope: InstallScope, projectPath: string | null, lockScope = false) {
    if (!selectedSkill) return;
    openInstallModal([selectedSkill.name], { scope, projectPath, lockScope });
  }

  /** Uninstall from every agent of one location, after confirming. */
  function handleUninstallLocation(
    scope: InstallScope,
    projectPath: string | null,
    installs: InstallView[]
  ) {
    const skill = selectedSkill;
    if (!skill || installs.length === 0) return;
    const name = skill.name;
    const paths = installs.map((install) => install.path);
    const location =
      scope === "user"
        ? $t("scope.user")
        : ($userProjects.find((project) => project.path === projectPath)?.name ??
          baseName(projectPath ?? ""));
    void runAction(async () => {
      const confirmed = await confirm($t("locationPicker.uninstallConfirm", { name, location }), {
        title: $t("scope.uninstall"),
        kind: "warning",
      });
      if (!confirmed) return;
      const result = await performAction((force) =>
        uninstallSkill({ name, targets: [], paths, force })
      );
      if (!result.applied && result.blockers.length > 0) {
        actionError = result.blockers.join("\n");
      }
    });
  }

  function handleTargetAction(install: InstallView, action: TargetAction) {
    const skill = selectedSkill;
    if (!skill) return;
    const name = skill.name;
    void runAction(async () => {
      let result;
      switch (action) {
        case "reinstall":
          if (install.mode === "symlink") {
            const targets = install.agentIds.map((agentId) => ({
              scope: install.scope,
              projectPath: install.projectPath ?? null,
              agentId,
            }));
            result = await performAction((force) =>
              installSkill({ name, targets, mode: "symlink", force })
            );
            break;
          }
          result = await performAction((force) =>
            syncSkill(name, { kind: "push_targets", targets: [install.path], force })
          );
          break;
        case "push":
          result = await performAction((force) =>
            syncSkill(name, { kind: "push_targets", targets: [install.path], force })
          );
          break;
        case "adopt":
          result = await performAction((force) =>
            syncSkill(name, { kind: "adopt_target", path: install.path, force })
          );
          break;
      }
      if (result && !result.applied && result.blockers.length > 0) {
        actionError = result.blockers.join("\n");
      }
    });
  }

  function withForce(action: SyncAction, force: boolean): SyncAction {
    return action.kind === "accept_hub" ? action : { ...action, force };
  }

  function handleSync(action: SyncAction) {
    const skill = selectedSkill;
    if (!skill) return;
    const name = skill.name;
    void runAction(async () => {
      const result = await performAction((force) => syncSkill(name, withForce(action, force)));
      if (!result.applied && result.blockers.length > 0) {
        actionError = result.blockers.join("\n");
      }
    });
  }

  function handleOpenDir() {
    if (!selectedSkill) return;
    openInFileManager(selectedSkill.hubPath).catch((error) => (actionError = String(error)));
  }

  function handleCheckSource() {
    if (!selectedSkill) return;
    void checkSourceUpdates([selectedSkill.name]).then((updates) => {
      const failed = updates.find((update) => update.error);
      if (failed?.error) actionError = failed.error;
    });
  }

  async function handleRemove(removeInstalls: boolean) {
    if (!selectedSkill) return;
    await removeHubSkill(selectedSkill.name, removeInstalls);
  }

  async function handleRemoved() {
    await refreshHub();
    navigate({ skill: null });
  }
</script>

<section class="flex h-full min-h-0 min-w-0 flex-col overflow-hidden">
  {#if migrationNotice}
    <div
      class="border-base-300 bg-base-200 text-base-content flex items-start gap-2 border-b px-4 py-2 text-xs"
    >
      <Info size={14} class="text-primary mt-0.5 shrink-0" />
      <div class="min-w-0 flex-1">
        {#if migrationNotice.kind === "errors"}
          <p>{$t("migration.errors", { count: migrationNotice.count })}</p>
          <ul class="text-base-content-muted mt-1 max-h-24 overflow-y-auto">
            {#each migrationNotice.details as detail}
              <li class="truncate" title={detail}>{detail}</li>
            {/each}
          </ul>
        {:else}
          <p>{$t("migration.notice", { count: migrationNotice.count })}</p>
        {/if}
      </div>
      <button
        class="text-base-content-muted hover:text-base-content shrink-0"
        type="button"
        onclick={() => (migrationDismissed = true)}
        title={$t("migration.dismiss")}
      >
        <X size={14} />
      </button>
    </div>
  {/if}

  <div
    class="grid min-h-0 flex-1 grid-cols-[18rem_minmax(0,1fr)] max-[1000px]:grid-cols-[15rem_minmax(0,1fr)]"
  >
    <SkillList
      skills={filteredSkills}
      totalCount={$hubSkills.length}
      {selectedName}
      loading={$hubLoading}
      error={$hubError}
      bind:search
      {filter}
      onSelect={handleSelect}
      onRefresh={() => refreshHub().catch(console.error)}
      onScan={() => openImportModal({ tab: "folder" })}
      onFilterChange={(next) => navigate({ filter: next })}
    />

    {#if selectedSkill}
      {#key selectedSkill.name}
        <SkillDetail
          skill={selectedSkill}
          projects={$userProjects}
          agents={$agentsById}
          homePath={$homePath}
          {busy}
          checkingSource={$sourceChecking}
          {actionError}
          onInstall={handleInstall}
          onUninstallLocation={handleUninstallLocation}
          onManageLocations={() => openLocationPickerModal(selectedSkill.name)}
          onTargetAction={handleTargetAction}
          onSync={handleSync}
          onRemove={() => (removeModalOpen = true)}
          onOpenDir={handleOpenDir}
          onCheckSource={handleCheckSource}
        />
      {/key}
    {:else}
      <div class="flex min-h-0 flex-col">
        <header class="border-base-300 h-12 flex-none border-b" data-window-drag-region></header>
        <div
          class="text-base-content-muted flex flex-1 items-center justify-center p-8 text-center text-sm"
        >
          {$t("library.selectHint")}
        </div>
      </div>
    {/if}
  </div>
</section>

<RemoveSkillModal
  bind:open={removeModalOpen}
  name={selectedSkill?.name ?? ""}
  installCount={selectedSkill?.installs.length ?? 0}
  remove={handleRemove}
  onRemoved={handleRemoved}
/>
