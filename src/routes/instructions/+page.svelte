<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import { confirm } from "@tauri-apps/plugin-dialog";
  import InstructionList from "$lib/components/instructions/InstructionList.svelte";
  import InstructionDetail from "$lib/components/instructions/InstructionDetail.svelte";
  import InstructionFileDetail from "$lib/components/instructions/InstructionFileDetail.svelte";
  import type { TargetAction } from "$lib/components/library/DriftPanel.svelte";
  import RemoveSkillModal from "$lib/components/RemoveSkillModal.svelte";
  import { t } from "$lib/i18n";
  import {
    buildInstructionsHref,
    getAppLocation,
    type LibraryFilter,
  } from "$lib/navigation/app-shell";
  import type { InstallScope, InstallView, SyncAction } from "$lib/api/hub";
  import {
    installInstruction,
    removeInstruction,
    syncInstruction,
    uninstallInstruction,
    type AgentFileView,
    type InstructionView,
  } from "$lib/api/instructions";
  import { openInFileManager } from "$lib/api/skills";
  import { homePath } from "$lib/stores/env";
  import { agentsById } from "$lib/stores/hub";
  import {
    instructionFiles,
    instructions,
    instructionsError,
    instructionsLoading,
    refreshInstructions,
  } from "$lib/stores/instructions";
  import {
    openImportInstructionModal,
    openInstallModal,
    openLocationPickerModal,
    performInstructionAction,
  } from "$lib/stores/modals";
  import { userProjects } from "$lib/stores/user-projects";
  import { baseName } from "$lib/scopes";

  let search = $state("");
  let busy = $state(false);
  let actionError = $state("");
  let removeModalOpen = $state(false);

  const location = $derived(getAppLocation(page.url));
  const selectedId = $derived(location.instruction);
  const selectedFilePath = $derived(location.instructionFile);
  const filter = $derived(location.filter);
  const needle = $derived(search.trim().toLowerCase());

  const filteredTemplates = $derived.by(() =>
    $instructions.filter((item) => {
      if (needle) {
        const haystack = `${item.name} ${item.description ?? ""}`.toLowerCase();
        if (!haystack.includes(needle)) return false;
      }
      switch (filter) {
        case "changed":
          return item.hasDrift;
        case "uninstalled":
          return item.installs.length === 0;
        default:
          return true;
      }
    })
  );

  const filteredFiles = $derived.by(() =>
    $instructionFiles.filter((file) => {
      if (needle) {
        const haystack = `${file.fileName} ${file.path} ${file.template?.name ?? ""}`.toLowerCase();
        if (!haystack.includes(needle)) return false;
      }
      switch (filter) {
        case "changed":
          return Boolean(file.template && file.template.state !== "in_sync");
        case "uninstalled":
          return !file.template;
        default:
          return true;
      }
    })
  );

  const selected = $derived<InstructionView | null>(
    selectedId ? ($instructions.find((item) => item.id === selectedId) ?? null) : null
  );
  const selectedFile = $derived<AgentFileView | null>(
    selectedFilePath
      ? ($instructionFiles.find((file) => file.path === selectedFilePath) ?? null)
      : null
  );

  onMount(() => {
    refreshInstructions().catch(console.error);
  });

  function navigate(options: { id?: string | null; file?: string | null; filter?: LibraryFilter }) {
    const keepSelection = options.id === undefined && options.file === undefined;
    const href = buildInstructionsHref({
      id: keepSelection ? selectedId : options.id,
      file: keepSelection ? selectedFilePath : options.file,
      filter: options.filter ?? filter,
    });
    goto(href, { replaceState: true, keepFocus: true, noScroll: true });
  }

  function selectTemplate(id: string) {
    actionError = "";
    navigate({ id, file: null });
  }

  function selectFile(path: string) {
    actionError = "";
    navigate({ id: null, file: path });
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
      await refreshInstructions().catch(console.error);
    }
  }

  function handleInstall(scope: InstallScope, projectPath: string | null, lockScope = false) {
    if (!selected) return;
    openInstallModal([selected.id], { scope, projectPath, lockScope, kind: "instruction" });
  }

  /** Uninstall from every agent of one location, after confirming. */
  function handleUninstallLocation(
    scope: InstallScope,
    projectPath: string | null,
    installs: InstallView[]
  ) {
    const item = selected;
    if (!item || installs.length === 0) return;
    const id = item.id;
    const paths = installs.map((install) => install.path);
    const location =
      scope === "user"
        ? $t("scope.user")
        : ($userProjects.find((project) => project.path === projectPath)?.name ??
          baseName(projectPath ?? ""));
    void runAction(async () => {
      const confirmed = await confirm(
        $t("locationPicker.uninstallConfirm", { name: item.name, location }),
        { title: $t("scope.uninstall"), kind: "warning" }
      );
      if (!confirmed) return;
      const result = await performInstructionAction((force) =>
        uninstallInstruction({ name: id, targets: [], paths, force })
      );
      if (!result.applied && result.blockers.length > 0) {
        actionError = result.blockers.join("\n");
      }
    });
  }

  function handleTargetAction(install: InstallView, action: TargetAction) {
    const item = selected;
    if (!item) return;
    const name = item.id;
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
            result = await performInstructionAction((force) =>
              installInstruction({ name, targets, mode: "symlink", force })
            );
            break;
          }
          result = await performInstructionAction((force) =>
            syncInstruction(name, { kind: "push_targets", targets: [install.path], force })
          );
          break;
        case "push":
          result = await performInstructionAction((force) =>
            syncInstruction(name, { kind: "push_targets", targets: [install.path], force })
          );
          break;
        case "adopt":
          result = await performInstructionAction((force) =>
            syncInstruction(name, { kind: "adopt_target", path: install.path, force })
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
    const item = selected;
    if (!item) return;
    const id = item.id;
    void runAction(async () => {
      const result = await performInstructionAction((force) =>
        syncInstruction(id, withForce(action, force))
      );
      if (!result.applied && result.blockers.length > 0) {
        actionError = result.blockers.join("\n");
      }
    });
  }

  function reveal(path: string) {
    openInFileManager(path).catch((error) => (actionError = String(error)));
  }

  /** Suggested template name for a promoted file: the project (or the user) and the file. */
  function promoteName(file: AgentFileView): string {
    const where = file.scope === "user" ? $t("scope.user") : baseName(file.projectPath ?? "");
    return `${where} ${file.fileName.replace(/\.[^.]+$/, "")}`;
  }

  async function handleRemove(removeInstalls: boolean) {
    if (!selected) return;
    await removeInstruction(selected.id, removeInstalls);
  }

  async function handleRemoved() {
    await refreshInstructions();
    navigate({ id: null, file: null });
  }
</script>

<section class="flex h-full min-h-0 min-w-0 flex-col overflow-hidden">
  <div
    class="grid min-h-0 flex-1 grid-cols-[18rem_minmax(0,1fr)] max-[1000px]:grid-cols-[15rem_minmax(0,1fr)]"
  >
    <InstructionList
      templates={filteredTemplates}
      files={filteredFiles}
      totalTemplates={$instructions.length}
      {selectedId}
      selectedFile={selectedFilePath}
      loading={$instructionsLoading}
      error={$instructionsError}
      bind:search
      {filter}
      onSelectTemplate={selectTemplate}
      onSelectFile={selectFile}
      onRefresh={() => refreshInstructions().catch(console.error)}
      onImport={() => openImportInstructionModal()}
      onFilterChange={(next) => navigate({ filter: next })}
    />

    {#if selected}
      {#key selected.id}
        <InstructionDetail
          instruction={selected}
          projects={$userProjects}
          agents={$agentsById}
          homePath={$homePath}
          {busy}
          {actionError}
          onInstall={handleInstall}
          onUninstallLocation={handleUninstallLocation}
          onManageLocations={() => openLocationPickerModal(selected.id, "instruction")}
          onTargetAction={handleTargetAction}
          onSync={handleSync}
          onRemove={() => (removeModalOpen = true)}
          onOpenFile={() => reveal(selected.hubPath)}
        />
      {/key}
    {:else if selectedFile}
      {#key selectedFile.path}
        <InstructionFileDetail
          file={selectedFile}
          agents={$agentsById}
          {busy}
          {actionError}
          onOpenTemplate={selectTemplate}
          onPromote={() =>
            openImportInstructionModal({
              paths: [selectedFile.path],
              name: promoteName(selectedFile),
            })}
          onOpenFile={() => reveal(selectedFile.path)}
        />
      {/key}
    {:else}
      <div class="flex min-h-0 flex-col">
        <header class="border-base-300 h-12 flex-none border-b" data-window-drag-region></header>
        <div
          class="text-base-content-muted flex flex-1 items-center justify-center p-8 text-center text-sm"
        >
          {$t("instructions.selectHint")}
        </div>
      </div>
    {/if}
  </div>
</section>

<RemoveSkillModal
  bind:open={removeModalOpen}
  name={selected?.name ?? ""}
  installCount={selected?.installs.length ?? 0}
  kind="instruction"
  remove={handleRemove}
  onRemoved={handleRemoved}
/>
