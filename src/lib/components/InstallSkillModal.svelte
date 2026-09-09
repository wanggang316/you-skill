<script lang="ts">
  import { AlertCircle } from "@lucide/svelte";
  import { get } from "svelte/store";
  import Modal from "$lib/components/ui/Modal.svelte";
  import PrimaryActionButton from "$lib/components/ui/PrimaryActionButton.svelte";
  import SegmentedTabs from "$lib/components/ui/SegmentedTabs.svelte";
  import SelectField from "$lib/components/ui/SelectField.svelte";
  import AgentPicker from "./AgentPicker.svelte";
  import { t } from "../i18n";
  import {
    installSkill,
    installedAgentIds,
    uninstallSkill,
    type InstallMode,
    type InstallScope,
    type InstallTargetSpec,
    type ScopeRef,
  } from "../api/hub";
  import { installInstruction, uninstallInstruction } from "../api/instructions";
  import { agents, hubSkillsByName, refreshHub } from "../stores/hub";
  import { instructionsByName, refreshInstructions } from "../stores/instructions";
  import {
    closeInstallModal,
    installModal,
    performAction,
    performInstructionAction,
  } from "../stores/modals";
  import { settings } from "../stores/settings";
  import { userProjects } from "../stores/user-projects";
  import { baseName } from "../scopes";

  let open = $state(false);
  let scope = $state<InstallScope>("user");
  let projectPath = $state<string | null>(null);
  /** Places fixed by the caller; takes precedence over `scope` and `projectPath`. */
  let lockedTargets = $state<ScopeRef[]>([]);
  let selectedIds = $state<string[]>([]);
  let mode = $state<InstallMode>("copy");
  let applying = $state(false);
  let error = $state("");

  const skillNames = $derived($installModal.skillNames);
  const isInstruction = $derived($installModal.kind === "instruction");
  const isSingle = $derived(skillNames.length === 1);
  const locked = $derived($installModal.lockScope);
  const targets = $derived.by((): ScopeRef[] => {
    if (lockedTargets.length > 0) return lockedTargets;
    if (scope === "user") return [{ scope: "user", projectPath: null }];
    return projectPath ? [{ scope: "project", projectPath }] : [];
  });
  /** Agents are grouped by the directories of this scope; mixed targets use the project view. */
  const pickerScope = $derived<InstallScope>(
    targets.some((target) => target.scope === "project") ? "project" : "user"
  );
  const targetName = $derived.by(() => {
    if (targets.length > 1) return $t("install.locationCount", { count: targets.length });
    const target = targets[0];
    if (!target || target.scope === "user") return $t("scope.user");
    const path = target.projectPath ?? "";
    const project = $userProjects.find((item) => item.path === path);
    return project?.name ?? (path ? baseName(path) : "");
  });
  const title = $derived.by(() => {
    const name = skillNames[0] ?? "";
    const count = skillNames.length;
    if (locked) {
      return isSingle
        ? $t("install.titleTo", { name, target: targetName })
        : $t("install.titleMultiTo", { count, target: targetName });
    }
    return isSingle ? $t("install.title", { name }) : $t("install.titleMulti", { count });
  });

  /** Agents already installed; only meaningful for one skill going to one place. */
  const currentIds = $derived.by(() => {
    if (!isSingle || targets.length !== 1) return [] as string[];
    const view = isInstruction
      ? $instructionsByName.get(skillNames[0])
      : $hubSkillsByName.get(skillNames[0]);
    if (!view) return [] as string[];
    return installedAgentIds(view, targets[0].scope, targets[0].projectPath);
  });

  const hasProjects = $derived($userProjects.length > 0);
  const canApply = $derived(!applying && targets.length > 0 && hasChanges());

  const refresh = () => (isInstruction ? refreshInstructions() : refreshHub());

  function hasChanges(): boolean {
    const current = new Set(currentIds);
    const selected = new Set(selectedIds);
    if (selected.size !== current.size) return true;
    for (const id of selected) if (!current.has(id)) return true;
    return false;
  }

  $effect(() => {
    const state = $installModal;
    if (state.open && !open) {
      lockedTargets = state.lockScope ? state.targets : [];
      scope = lockedTargets[0]?.scope ?? state.initialScope;
      projectPath =
        scope === "project"
          ? (lockedTargets[0]?.projectPath ??
            state.initialProjectPath ??
            get(userProjects)[0]?.path ??
            null)
          : null;
      mode = get(settings).sync_mode === "symlink" ? "symlink" : "copy";
      error = "";
      applying = false;
      selectedIds = [...currentIds];
    }
    open = state.open;
  });

  // Re-seed the selection whenever the scope/project changes.
  $effect(() => {
    scope;
    projectPath;
    lockedTargets;
    selectedIds = [...currentIds];
  });

  function handleClose() {
    open = false;
    closeInstallModal();
  }

  function handleScopeChange(value: string) {
    scope = value === "project" ? "project" : "user";
    if (scope === "project" && !projectPath) {
      projectPath = get(userProjects)[0]?.path ?? null;
    }
    if (scope === "user") {
      projectPath = null;
    }
  }

  function specsFor(ids: string[], target: ScopeRef): InstallTargetSpec[] {
    return ids.map((agentId) => ({
      scope: target.scope,
      projectPath: target.projectPath,
      agentId,
    }));
  }

  async function handleApply() {
    if (!canApply) return;
    applying = true;
    error = "";
    const failures: string[] = [];
    try {
      const single = isSingle && targets.length === 1;
      for (const name of skillNames) {
        for (const target of targets) {
          const view = isInstruction
            ? get(instructionsByName).get(name)
            : get(hubSkillsByName).get(name);
          const current = view ? installedAgentIds(view, target.scope, target.projectPath) : [];
          const add = selectedIds.filter((id) => !current.includes(id));
          const remove = single ? current.filter((id) => !selectedIds.includes(id)) : [];

          if (add.length > 0) {
            const request = { name, targets: specsFor(add, target), mode };
            const result = isInstruction
              ? await performInstructionAction((force) => installInstruction({ ...request, force }))
              : await performAction((force) => installSkill({ ...request, force }));
            if (!result.applied) failures.push(`${name}: ${result.blockers.join("; ")}`);
          }
          if (remove.length > 0) {
            const request = { name, targets: specsFor(remove, target) };
            const result = isInstruction
              ? await performInstructionAction((force) =>
                  uninstallInstruction({ ...request, force })
                )
              : await performAction((force) => uninstallSkill({ ...request, force }));
            if (!result.applied) failures.push(`${name}: ${result.blockers.join("; ")}`);
          }
        }
      }
      await refresh();
      if (failures.length > 0) {
        error = failures.join("\n");
        return;
      }
      handleClose();
    } catch (err) {
      error = String(err);
      await refresh().catch(console.error);
    } finally {
      applying = false;
    }
  }
</script>

<Modal bind:open {title} onClose={handleClose} containerClass="max-w-xl">
  <div class="space-y-5 px-6 pt-2 pb-6">
    {#if !locked}
      <div class="space-y-3">
        <SegmentedTabs
          items={[
            { value: "user", label: $t("install.scope.user") },
            { value: "project", label: $t("install.scope.project") },
          ]}
          value={scope}
          onChange={handleScopeChange}
          fullWidth={true}
        />
        {#if scope === "project"}
          {#if hasProjects}
            <SelectField bind:value={projectPath} disabled={applying} className="w-full">
              {#each $userProjects as project (project.path)}
                <option value={project.path}>{project.name}</option>
              {/each}
            </SelectField>
          {:else}
            <p class="text-base-content-muted text-sm">{$t("install.noProjects")}</p>
          {/if}
        {/if}
      </div>
    {/if}

    <AgentPicker
      agents={$agents}
      scope={pickerScope}
      target={isInstruction ? "instructions" : "skills"}
      bind:selectedIds
      disabled={applying}
    />

    <div class="flex items-center justify-between gap-3">
      <div class="text-base-content-muted text-[13px]">
        <p>{$t("install.mode")}</p>
        {#if isSingle && currentIds.length > 0}
          <p class="text-base-content-faint text-xs">{$t("install.removeHint")}</p>
        {/if}
      </div>
      <SelectField bind:value={mode} disabled={applying}>
        <option value="copy">{$t("install.mode.copy")}</option>
        <option value="symlink">{$t("install.mode.symlink")}</option>
      </SelectField>
    </div>

    {#if error}
      <div class="text-error flex items-start gap-2 text-sm whitespace-pre-wrap">
        <AlertCircle size={16} class="mt-0.5 shrink-0" />
        <span>{error}</span>
      </div>
    {/if}
  </div>
  {#snippet footer()}
    <button
      class="border-base-300 text-base-content hover:bg-base-200 rounded-xl border px-4 py-2 text-sm transition"
      type="button"
      onclick={handleClose}
      disabled={applying}
    >
      {$t("common.cancel")}
    </button>
    <PrimaryActionButton
      onclick={handleApply}
      disabled={!canApply}
      loading={applying}
      loadingText={$t("install.applying")}
    >
      {hasChanges() ? $t("install.confirm") : $t("install.noChanges")}
    </PrimaryActionButton>
  {/snippet}
</Modal>
