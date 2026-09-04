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
  } from "../api/hub";
  import { agents, hubSkillsByName, refreshHub } from "../stores/hub";
  import { closeInstallModal, installModal, performAction } from "../stores/modals";
  import { settings } from "../stores/settings";
  import { userProjects } from "../stores/user-projects";

  let open = $state(false);
  let scope = $state<InstallScope>("user");
  let projectPath = $state<string | null>(null);
  let selectedIds = $state<string[]>([]);
  let mode = $state<InstallMode>("copy");
  let applying = $state(false);
  let error = $state("");

  const skillNames = $derived($installModal.skillNames);
  const isSingle = $derived(skillNames.length === 1);
  const title = $derived(
    isSingle
      ? $t("install.title", { name: skillNames[0] ?? "" })
      : $t("install.titleMulti", { count: skillNames.length })
  );

  const currentIds = $derived.by(() => {
    if (!isSingle) return [] as string[];
    const view = $hubSkillsByName.get(skillNames[0]);
    if (!view) return [] as string[];
    if (scope === "project" && !projectPath) return [] as string[];
    return installedAgentIds(view, scope, projectPath);
  });

  const hasProjects = $derived($userProjects.length > 0);
  const canApply = $derived(
    !applying && (scope === "user" || Boolean(projectPath)) && hasChanges()
  );

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
      scope = state.initialScope;
      projectPath =
        state.initialScope === "project"
          ? (state.initialProjectPath ?? get(userProjects)[0]?.path ?? null)
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

  function specsFor(ids: string[]): InstallTargetSpec[] {
    return ids.map((agentId) => ({
      scope,
      projectPath: scope === "project" ? projectPath : null,
      agentId,
    }));
  }

  async function handleApply() {
    if (!canApply) return;
    applying = true;
    error = "";
    const failures: string[] = [];
    try {
      for (const name of skillNames) {
        const view = get(hubSkillsByName).get(name);
        const current = view ? installedAgentIds(view, scope, projectPath) : [];
        const add = selectedIds.filter((id) => !current.includes(id));
        const remove = isSingle ? current.filter((id) => !selectedIds.includes(id)) : [];

        if (add.length > 0) {
          const result = await performAction((force) =>
            installSkill({ name, targets: specsFor(add), mode, force })
          );
          if (!result.applied) failures.push(`${name}: ${result.blockers.join("; ")}`);
        }
        if (remove.length > 0) {
          const result = await performAction((force) =>
            uninstallSkill({ name, targets: specsFor(remove), force })
          );
          if (!result.applied) failures.push(`${name}: ${result.blockers.join("; ")}`);
        }
      }
      await refreshHub();
      if (failures.length > 0) {
        error = failures.join("\n");
        return;
      }
      handleClose();
    } catch (err) {
      error = String(err);
      await refreshHub().catch(console.error);
    } finally {
      applying = false;
    }
  }
</script>

<Modal bind:open {title} onClose={handleClose} containerClass="max-w-xl">
  <div class="space-y-5 px-6 pt-2 pb-6">
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

    <AgentPicker agents={$agents} {scope} bind:selectedIds disabled={applying} />

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
