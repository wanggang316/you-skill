<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import InstructionList from "$lib/components/instructions/InstructionList.svelte";
  import InstructionDetail from "$lib/components/instructions/InstructionDetail.svelte";
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
    type InstructionView,
  } from "$lib/api/instructions";
  import { openInFileManager } from "$lib/api/skills";
  import { homePath } from "$lib/stores/env";
  import { agentsById } from "$lib/stores/hub";
  import {
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

  let search = $state("");
  let busy = $state(false);
  let actionError = $state("");
  let removeModalOpen = $state(false);

  const location = $derived(getAppLocation(page.url));
  const selectedName = $derived(location.instruction);
  const filter = $derived(location.filter);

  const filtered = $derived.by(() => {
    const needle = search.trim().toLowerCase();
    return $instructions.filter((item) => {
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
    });
  });

  const selected = $derived<InstructionView | null>(
    selectedName ? ($instructions.find((item) => item.name === selectedName) ?? null) : null
  );

  onMount(() => {
    refreshInstructions().catch(console.error);
  });

  function navigate(options: { name?: string | null; filter?: LibraryFilter }) {
    const href = buildInstructionsHref({
      name: options.name === undefined ? selectedName : options.name,
      filter: options.filter ?? filter,
    });
    goto(href, { replaceState: true, keepFocus: true, noScroll: true });
  }

  function handleSelect(name: string) {
    actionError = "";
    navigate({ name });
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
    openInstallModal([selected.name], { scope, projectPath, lockScope, kind: "instruction" });
  }

  function handleTargetAction(install: InstallView, action: TargetAction) {
    const item = selected;
    if (!item) return;
    const name = item.name;
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
    const name = item.name;
    void runAction(async () => {
      const result = await performInstructionAction((force) =>
        syncInstruction(name, withForce(action, force))
      );
      if (!result.applied && result.blockers.length > 0) {
        actionError = result.blockers.join("\n");
      }
    });
  }

  function handleOpenFile() {
    if (!selected) return;
    openInFileManager(selected.hubPath).catch((error) => (actionError = String(error)));
  }

  async function handleRemove(removeInstalls: boolean) {
    if (!selected) return;
    await removeInstruction(selected.name, removeInstalls);
  }

  async function handleRemoved() {
    await refreshInstructions();
    navigate({ name: null });
  }
</script>

<section class="flex h-full min-h-0 min-w-0 flex-col overflow-hidden">
  <div
    class="grid min-h-0 flex-1 grid-cols-[18rem_minmax(0,1fr)] max-[1000px]:grid-cols-[15rem_minmax(0,1fr)]"
  >
    <InstructionList
      items={filtered}
      totalCount={$instructions.length}
      {selectedName}
      loading={$instructionsLoading}
      error={$instructionsError}
      bind:search
      {filter}
      onSelect={handleSelect}
      onRefresh={() => refreshInstructions().catch(console.error)}
      onImport={openImportInstructionModal}
      onFilterChange={(next) => navigate({ filter: next })}
    />

    {#if selected}
      {#key selected.name}
        <InstructionDetail
          instruction={selected}
          projects={$userProjects}
          agents={$agentsById}
          homePath={$homePath}
          {busy}
          {actionError}
          onInstall={handleInstall}
          onManageLocations={() => openLocationPickerModal(selected.name, "instruction")}
          onTargetAction={handleTargetAction}
          onSync={handleSync}
          onRemove={() => (removeModalOpen = true)}
          onOpenFile={handleOpenFile}
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
